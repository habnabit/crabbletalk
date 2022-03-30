use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
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
    let mac = crabbletalk::addr::Mac::new_random();
    println!("afpd starting up on {:?}", mac);
    let (mut aarp_stack, mut atalk_rx) = crabbletalk::aarp::AarpStackHandle::spawn(mac);

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = sock.recv_from(&mut buf) => { r }
            atalk = atalk_rx.recv() => {
                match &atalk {
                    Some(p) => {
                        println!("oh no {:?}", p);
                        sock.send(&p.0[..]).await?;
                    }
                    None => {
                        println!("oh no bye??");
                        break;
                    }
                }
                continue
            }
        }?;
        let data = &buf[..n_read];
        aarp_stack.process_ethernet(data).await?;
    }

    std::fs::remove_file(&sock_path).with_context(|| format!("whilst deleting {:?}", sock_path))?;

    Ok(())
}
