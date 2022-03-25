use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use crabbletalk::link::decode_appletalk;
use pnet::packet::Packet;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let sock_path: PathBuf = args[1].parse()?;
    let router_path: PathBuf = args[2].parse()?;
    let sock = tokio::net::UnixDatagram::bind(&sock_path)?;
    sock.connect(&router_path)?;
    sock.send(b"").await?;
    let mut buf = vec![0u8; 1600];

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = sock.recv_from(&mut buf) => { r }
        }?;
        let data = &buf[..n_read];
        if let Some(p) = decode_appletalk(data) {
            println!("read {} bytes: {:#?}\n  {:?}", n_read, p, p.payload());
        }
    }

    std::fs::remove_file(&sock_path).with_context(|| format!("whilst deleting {:?}", sock_path))?;

    Ok(())
}
