use anyhow::{anyhow, Context, Result};
use std::io::ErrorKind;
use std::os::unix::net::UnixDatagram;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use wayland_client::protocol::{wl_pointer, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

use crate::ipc::{self, Command, BUTTON_STATE_PRESSED};

struct State {
    seat: Option<wl_seat::WlSeat>,
    manager: Option<ZwlrVirtualPointerManagerV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" => {
                    state.seat =
                        Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(9), qh, ()));
                }
                "zwlr_virtual_pointer_manager_v1" => {
                    state.manager = Some(registry.bind::<ZwlrVirtualPointerManagerV1, _, _>(
                        name,
                        version.min(2),
                        qh,
                        (),
                    ));
                }
                _ => {}
            }
        }
    }
}

macro_rules! ignore_dispatch {
    ($iface:ty) => {
        impl Dispatch<$iface, ()> for State {
            fn event(
                _: &mut Self,
                _: &$iface,
                _: <$iface as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

ignore_dispatch!(wl_seat::WlSeat);
ignore_dispatch!(ZwlrVirtualPointerManagerV1);
ignore_dispatch!(ZwlrVirtualPointerV1);

struct Daemon {
    conn: Connection,
    queue: wayland_client::EventQueue<State>,
    state: State,
    start: Instant,
}

impl Daemon {
    fn new() -> Result<Self> {
        let conn = Connection::connect_to_env().context("failed to connect to Wayland display")?;
        let display = conn.display();
        let mut queue = conn.new_event_queue::<State>();
        let qh = queue.handle();
        let _registry = display.get_registry(&qh, ());
        let mut state = State {
            seat: None,
            manager: None,
        };
        queue
            .roundtrip(&mut state)
            .context("Wayland roundtrip failed")?;
        if state.seat.is_none() || state.manager.is_none() {
            return Err(anyhow!("virtual pointer manager or seat is unavailable"));
        }
        Ok(Self {
            conn,
            queue,
            state,
            start: Instant::now(),
        })
    }

    fn time_ms(&self) -> u32 {
        self.start.elapsed().as_millis() as u32
    }

    fn new_pointer(&self) -> ZwlrVirtualPointerV1 {
        let manager = self.state.manager.as_ref().expect("manager bound");
        let seat = self.state.seat.as_ref().expect("seat bound");
        manager.create_virtual_pointer(Some(seat), &self.queue.handle(), ())
    }

    fn flush(&self) -> Result<()> {
        self.conn.flush().context("failed to flush connection")?;
        Ok(())
    }

    fn emit_scroll(&self, vertical: i32, horizontal: i32) -> Result<()> {
        if vertical == 0 && horizontal == 0 {
            return Ok(());
        }
        let t = self.time_ms();
        let pointer = self.new_pointer();
        pointer.axis_source(wl_pointer::AxisSource::Wheel);
        if vertical != 0 {
            pointer.axis_discrete(
                t,
                wl_pointer::Axis::VerticalScroll,
                15.0 * vertical as f64,
                vertical,
            );
        }
        if horizontal != 0 {
            pointer.axis_discrete(
                t,
                wl_pointer::Axis::HorizontalScroll,
                15.0 * horizontal as f64,
                horizontal,
            );
        }
        pointer.frame();
        self.flush()?;
        pointer.destroy();
        Ok(())
    }

    fn emit_motion(&self, dx: i32, dy: i32) -> Result<()> {
        let t = self.time_ms();
        let pointer = self.new_pointer();
        pointer.motion(t, dx as f64, dy as f64);
        pointer.frame();
        self.flush()?;
        pointer.destroy();
        Ok(())
    }

    fn emit_button(&self, button: u32, state: u32) -> Result<()> {
        let t = self.time_ms();
        let pointer = self.new_pointer();
        let bs = if state == BUTTON_STATE_PRESSED {
            wl_pointer::ButtonState::Pressed
        } else {
            wl_pointer::ButtonState::Released
        };
        pointer.button(t, button, bs);
        pointer.frame();
        self.flush()?;
        pointer.destroy();
        Ok(())
    }

    fn emit_click(&self, button: u32) -> Result<()> {
        let t = self.time_ms();
        let pointer = self.new_pointer();
        pointer.button(t, button, wl_pointer::ButtonState::Pressed);
        pointer.frame();
        pointer.button(t, button, wl_pointer::ButtonState::Released);
        pointer.frame();
        self.flush()?;
        pointer.destroy();
        Ok(())
    }

    fn dispatch(&self, cmd: Command) -> Result<()> {
        match cmd {
            Command::Scroll {
                vertical,
                horizontal,
            } => self.emit_scroll(vertical, horizontal),
            Command::Motion { dx, dy } => self.emit_motion(dx, dy),
            Command::Button { button, state } => self.emit_button(button, state),
            Command::Click { button } => self.emit_click(button),
        }
    }
}

pub fn run() -> Result<()> {
    let path = ipc::socket_path()?;
    let _ = std::fs::remove_file(&path);
    let sock =
        UnixDatagram::bind(&path).with_context(|| format!("failed to bind {}", path.display()))?;
    sock.set_read_timeout(Some(Duration::from_millis(200)))?;

    let terminated = Arc::new(AtomicBool::new(false));
    for sig in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register(sig, terminated.clone())
            .with_context(|| format!("failed to install signal handler for {sig}"))?;
    }

    let daemon = Daemon::new()?;
    let mut buf = [0u8; 256];

    let result = loop {
        if terminated.load(Ordering::SeqCst) {
            break Ok(());
        }
        match sock.recv(&mut buf) {
            Ok(n) => match ipc::decode(&buf[..n]) {
                Ok(cmd) => {
                    eprintln!("recv: {cmd:?}");
                    if let Err(e) = daemon.dispatch(cmd) {
                        eprintln!("dispatch error: {e:#}");
                    }
                }
                Err(e) => eprintln!("decode error: {e:#}"),
            },
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(e) => break Err(anyhow::Error::from(e).context("recv failed")),
        }
    };

    let _ = std::fs::remove_file(&path);
    result
}
