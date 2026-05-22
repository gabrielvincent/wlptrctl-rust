mod client;
mod daemon;
mod ipc;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

use ipc::{Command as IpcCommand, BUTTON_STATE_PRESSED, BUTTON_STATE_RELEASED};

// linux/input-event-codes.h
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

#[derive(Parser)]
#[command(name = "wlptrctl", about = "Wayland virtual pointer control")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the Wayland-side daemon.
    Daemon,
    /// Emit discrete scroll wheel steps.
    #[command(allow_negative_numbers = true)]
    Scroll { vertical: i32, horizontal: i32 },
    /// Move the virtual pointer by a relative offset.
    #[command(allow_negative_numbers = true)]
    Motion { dx: i32, dy: i32 },
    /// Press or release a button (left/right/middle or a numeric code).
    Button {
        button: String,
        #[arg(value_parser = parse_button_state)]
        state: u32,
    },
    /// Press and release a button (left/right/middle or a numeric code).
    Click { button: String },
}

fn parse_button(arg: &str) -> Result<u32> {
    Ok(match arg {
        "left" => BTN_LEFT,
        "right" => BTN_RIGHT,
        "middle" => BTN_MIDDLE,
        other => other
            .parse::<u32>()
            .map_err(|_| anyhow!("invalid button: {arg}"))?,
    })
}

fn parse_button_state(arg: &str) -> Result<u32, String> {
    match arg {
        "press" => Ok(BUTTON_STATE_PRESSED),
        "release" => Ok(BUTTON_STATE_RELEASED),
        _ => Err("button state must be 'press' or 'release'".into()),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Daemon => daemon::run(),
        Cmd::Scroll {
            vertical,
            horizontal,
        } => {
            if vertical == 0 && horizontal == 0 {
                return Ok(());
            }
            client::send(&IpcCommand::Scroll {
                vertical,
                horizontal,
            })
        }
        Cmd::Motion { dx, dy } => {
            if dx == 0 && dy == 0 {
                return Ok(());
            }
            client::send(&IpcCommand::Motion { dx, dy })
        }
        Cmd::Button { button, state } => {
            let button = parse_button(&button)?;
            client::send(&IpcCommand::Button { button, state })
        }
        Cmd::Click { button } => {
            let button = parse_button(&button)?;
            client::send(&IpcCommand::Click { button })
        }
    }
}
