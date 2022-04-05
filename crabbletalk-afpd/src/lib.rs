use anyhow::{Context, Result};
use std::{
    os::unix::net::UnixDatagram,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

pub struct UnlinkOnDrop(PathBuf);

impl UnlinkOnDrop {
    pub fn new(p: PathBuf) -> Self {
        Self(p)
    }
}

impl Drop for UnlinkOnDrop {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0).with_context(|| format!("whilst deleting {:?}", self.0))
        {
            Ok(()) => {}
            Err(_e) => {
                // welp
            }
        }
    }
}

pub fn anonymous_datagram_client(
    name: &str,
    loc: Option<&Path>,
) -> Result<(UnixDatagram, TempDir)> {
    let client_dir = {
        let mut builder = tempfile::Builder::new();
        let prefix = format!("{}_p{}_", name, std::process::id());
        builder.prefix(&prefix);
        match loc {
            Some(loc) => builder.tempdir_in(loc),
            None => builder.tempdir(),
        }
    }
    .context("whilst making a tempdir")?;
    let client_sock = client_dir.path().join("id.sock");
    let sock = UnixDatagram::bind(&client_sock)
        .with_context(|| format!("whilst binding to {:?}", client_sock))?;
    Ok((sock, client_dir))
}
