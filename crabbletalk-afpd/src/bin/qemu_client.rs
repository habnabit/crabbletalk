use std::{ffi::CString, os::unix::prelude::IntoRawFd, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use pnet::util::MacAddr;

fn new_mac() -> Result<MacAddr> {
    let mut nic = [0u8; 3];
    getrandom::getrandom(&mut nic[..])?;
    Ok(MacAddr::new(0x52, 0x54, 0, nic[0], nic[1], nic[2]))
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    tmpdir: Option<PathBuf>,
    #[clap(short = 'q', long, default_value = "qemu-system-ppc")]
    qemu: String,
    router_path: PathBuf,
    #[clap(raw = true)]
    remainder: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (sock, _unlinker) = crabbletalk_afpd::anonymous_datagram_client(
        "crabbletalk_qemu_client",
        cli.tmpdir.as_ref().map(|x| x.as_ref()),
    )?;
    sock.connect(&cli.router_path)
        .with_context(|| format!("whilst connecting to {:?}", cli.router_path))?;
    let sock_fd = sock.into_raw_fd();
    {
        use nix::fcntl::*;
        // clear FD_CLOEXEC
        fcntl(sock_fd, FcntlArg::F_SETFD(FdFlag::empty()))?;
    }
    let mut base_argv = vec![cli.qemu.as_str(), "-nic"];
    let mac = new_mac()?;
    let nic_arg = format!("tap,fd={},mac={}", sock_fd, mac);
    base_argv.push(&nic_arg);
    for arg in &cli.remainder {
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
