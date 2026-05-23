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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonAction {
    Press,
    Release,
    Click,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Run the Wayland-side daemon.
    Daemon,
    /// Emit discrete scroll wheel steps.
    #[command(allow_negative_numbers = true)]
    Scroll { vertical: i32, horizontal: i32 },
    /// Move the virtual pointer by a relative offset.
    #[command(allow_negative_numbers = true)]
    Move { dx: i32, dy: i32 },
    /// Press, release, or click a button (left/right/middle or a numeric code).
    Button {
        button: String,
        #[arg(value_parser = parse_button_action)]
        action: ButtonAction,
    },
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

fn parse_button_action(arg: &str) -> Result<ButtonAction, String> {
    match arg {
        "press" => Ok(ButtonAction::Press),
        "release" => Ok(ButtonAction::Release),
        "click" => Ok(ButtonAction::Click),
        _ => Err("button action must be 'press', 'release', or 'click'".into()),
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
        Cmd::Move { dx, dy } => {
            if dx == 0 && dy == 0 {
                return Ok(());
            }
            client::send(&IpcCommand::Motion { dx, dy })
        }
        Cmd::Button { button, action } => {
            let button = parse_button(&button)?;
            match action {
                ButtonAction::Press => client::send(&IpcCommand::Button {
                    button,
                    state: BUTTON_STATE_PRESSED,
                }),
                ButtonAction::Release => client::send(&IpcCommand::Button {
                    button,
                    state: BUTTON_STATE_RELEASED,
                }),
                ButtonAction::Click => client::send(&IpcCommand::Click { button }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ButtonAction, Cli, Cmd};
    use clap::Parser;

    #[test]
    fn parses_move_command() {
        let cli = Cli::try_parse_from(["wlptrctl", "move", "-10", "25"]).unwrap();
        match cli.cmd {
            Cmd::Move { dx, dy } => {
                assert_eq!(dx, -10);
                assert_eq!(dy, 25);
            }
            other => panic!("expected move command, got {other:?}"),
        }
    }

    #[test]
    fn parses_button_click_command() {
        let cli = Cli::try_parse_from(["wlptrctl", "button", "274", "click"]).unwrap();
        match cli.cmd {
            Cmd::Button { button, action } => {
                assert_eq!(button, "274");
                assert_eq!(action, ButtonAction::Click);
            }
            other => panic!("expected button command, got {other:?}"),
        }
    }

    #[test]
    fn rejects_old_motion_command() {
        assert!(Cli::try_parse_from(["wlptrctl", "motion", "1", "2"]).is_err());
    }

    #[test]
    fn rejects_old_click_command() {
        assert!(Cli::try_parse_from(["wlptrctl", "click", "left"]).is_err());
    }

    #[test]
    fn rejects_invalid_button_action() {
        assert!(Cli::try_parse_from(["wlptrctl", "button", "left", "hold"]).is_err());
    }
}
