use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
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
    bincode::serialize(cmd).context("failed to encode command")
}

pub fn decode(buf: &[u8]) -> Result<Command> {
    bincode::deserialize(buf).context("failed to decode command")
}
