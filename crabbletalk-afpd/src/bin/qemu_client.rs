use std::os::unix::net::UnixDatagram;
use std::os::unix::prelude::IntoRawFd;
use std::{ffi::CString, path::PathBuf};
use pnet::util::MacAddr;

use anyhow::{anyhow, Context, Result};

fn new_mac() -> Result<MacAddr> {
    let mut nic = [0u8; 3];
    getrandom::getrandom(&mut nic[..])?;
    Ok(MacAddr::new(0x52, 0x54, 0, nic[0], nic[1], nic[2]))
}

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
        "-device",
        "usb-kbd",
        "-device",
        "usb-mouse",
        "-nic",
    ];
    let mac = new_mac()?;
    let nic_arg = format!("tap,model=sungem,fd={},mac={}", sock_fd, mac);
    base_argv.push(&nic_arg);
    for arg in &args[2..] {
        base_argv.push(arg);
    }
    let argv: Vec<CString> = base_argv
        .into_iter()
        .map(|s| CString::new(s).map_err(|e| anyhow!("error allocating for {:?}", e)))
        .collect::<Result<_>>()
        .context("whilst allocating argv")?;
    nix::unistd::execvp(&argv[0], &argv[..]).context("whilst exec")?;
    unreachable!()
}
