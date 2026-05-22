use anyhow::{Context, Result};
use std::os::unix::net::UnixDatagram;

use crate::ipc::{self, Command};

pub fn send(cmd: &Command) -> Result<()> {
    let path = ipc::socket_path()?;
    let payload = ipc::encode(cmd)?;
    let sock = UnixDatagram::unbound().context("failed to create socket")?;
    sock.send_to(&payload, &path)
        .with_context(|| format!("failed to contact wlptrctl daemon at {}", path.display()))?;
    Ok(())
}
