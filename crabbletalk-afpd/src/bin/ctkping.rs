use std::{collections::BTreeSet, fs::File, os::unix::prelude::FromRawFd, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{unix::SocketAddr, UnixDatagram, UnixStream},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    cruats_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cruats_control = UnixStream::connect(&cli.cruats_path).await?;
    use sendfd::RecvWithFd;
    cruats_control.write_all(b"spam eggs").await?;
    let mut buffer = [0u8; 32];
    let mut fds = [-1; 2];
    cruats_control.readable().await?;
    let res = cruats_control.recv_with_fd(&mut buffer, &mut fds)?;
    println!("so what do we got {:?} {:?} {:?}", res, buffer, fds);
    let ddp = unsafe { std::os::unix::net::UnixDatagram::from_raw_fd(fds[0]) };
    let ddp = UnixDatagram::from_std(ddp)?;
    let res = ddp.recv_from(&mut buffer).await;
    println!("here we go again {:?} {:?}", res, buffer);
    let res = ddp.send(b"finally").await;
    println!("and again {:?}", res);
    Ok(())
}
