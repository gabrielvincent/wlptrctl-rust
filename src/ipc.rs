use anyhow::{anyhow, Context, Result};
use rkyv::{Archive, Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(compare(PartialEq))]
pub enum Command {
    Scroll { vertical: i32, horizontal: i32 },
    Motion { dx: i32, dy: i32 },
    Button { button: u32, state: u32 },
    Click { button: u32 },
}

pub const BUTTON_STATE_RELEASED: u32 = 0;
pub const BUTTON_STATE_PRESSED: u32 = 1;

pub fn socket_path() -> Result<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("XDG_RUNTIME_DIR is not set"))?;
    Ok(PathBuf::from(dir).join("wlptrctl.sock"))
}

pub fn encode(cmd: &Command) -> Result<Vec<u8>> {
    rkyv::to_bytes::<rkyv::rancor::Error>(cmd)
        .map(Vec::from)
        .context("failed to encode command")
}

pub fn decode(buf: &[u8]) -> Result<Command> {
    rkyv::from_bytes::<Command, rkyv::rancor::Error>(buf).context("failed to decode command")
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, Command, BUTTON_STATE_PRESSED};

    #[test]
    fn command_round_trips_through_rkyv() {
        let commands = [
            Command::Scroll {
                vertical: 1,
                horizontal: -1,
            },
            Command::Motion { dx: 50, dy: -25 },
            Command::Button {
                button: 0x110,
                state: BUTTON_STATE_PRESSED,
            },
            Command::Click { button: 0x112 },
        ];

        for command in commands {
            let encoded = encode(&command).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded, command);
        }
    }
}
