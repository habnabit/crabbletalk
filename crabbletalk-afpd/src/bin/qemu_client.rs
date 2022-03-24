use std::os::unix::net::UnixDatagram;
use std::os::unix::prelude::IntoRawFd;
use std::{ffi::CString, path::PathBuf};

use anyhow::{anyhow, Context, Result};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let server_path: PathBuf = args[1].parse()?;

    let client_dir = tempfile::Builder::new()
        .prefix(&format!("crabbletalk_{}", std::process::id()))
        .tempdir()
        .context("whilst making a tempdir")?;
    let client_sock = client_dir.path().join("id.sock");
    let sock = UnixDatagram::bind(&client_sock)
        .with_context(|| format!("whilst binding to {:?}", client_sock))?;
    sock.connect(&server_path)
        .with_context(|| format!("whilst connecting to {:?}", server_path))?;
    let sock_fd = sock.into_raw_fd();
    {
        use nix::fcntl::*;
        // clear FD_CLOEXEC
        fcntl(sock_fd, FcntlArg::F_SETFD(FdFlag::empty()))?;
    }
    let mut base_argv = vec![
        "qemu-system-ppc",
        "-M",
        "mac99",
        "-m",
        "512M",
        "-hda",
        "disk2.qcow2",
        "-device",
        "usb-kbd",
        "-device",
        "usb-tablet",
        "-nic",
    ];
    let nic_arg = format!("tap,model=sungem,fd={}", sock_fd);
    base_argv.push(&nic_arg);
    let argv: Vec<CString> = base_argv
        .into_iter()
        .map(|s| CString::new(s).map_err(|e| anyhow!("error allocating for {:?}", e)))
        .collect::<Result<_>>()
        .context("whilst allocating argv")?;
    nix::unistd::execvp(&argv[0], &argv[..]).context("whilst exec")?;
    unreachable!()
}
