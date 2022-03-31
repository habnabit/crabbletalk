use anyhow::{Context, Result};
use std::{os::unix::net::UnixDatagram, path::PathBuf};

pub struct UnlinkOnDrop(PathBuf);

impl UnlinkOnDrop {
    pub fn new(p: PathBuf) -> Self {
        Self(p)
    }
}

impl Drop for UnlinkOnDrop {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0).with_context(|| format!("whilst deleting {:?}", self.0)) {
            Ok(()) => {}
            Err(_e) => {
                // welp
            }
        }
    }
}

pub fn anonymous_datagram_client(name: &str) -> Result<(UnixDatagram, UnlinkOnDrop)> {
    let client_dir = tempfile::Builder::new()
        .prefix(&format!("{}_p{}_", name, std::process::id()))
        .tempdir()
        .context("whilst making a tempdir")?;
    let client_sock = client_dir.path().join("id.sock");
    let sock = UnixDatagram::bind(&client_sock)
        .with_context(|| format!("whilst binding to {:?}", client_sock))?;
    Ok((sock, UnlinkOnDrop(client_sock)))
}
