use std::{path::PathBuf, collections::BTreeSet};

use anyhow::{anyhow, Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let sock_path: PathBuf = args[1].parse()?;
    let listener = tokio::net::UnixDatagram::bind(&sock_path)?;
    let mut clients = BTreeSet::new();
    let mut buf = vec![0u8; 1600];

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = listener.recv_from(&mut buf) => { r }
        }?;
        let data = &buf[..n_read];
        println!("read {} bytes from {:?}", n_read, addr);
        if let Some(path) = addr.as_pathname() {
            if !clients.contains(path) {
                clients.insert(path.to_owned());
            }
            let mut to_remove = BTreeSet::new();
            for client in &clients {
                if client == path { continue }
                match listener.send_to(data, client).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("error {:?} on {:?}", e, client);
                        to_remove.insert(client.to_owned());
                    }
                }
            }
            if !to_remove.is_empty() {
                clients.retain(|c| !to_remove.contains(c));
            }
        }
    }

    std::fs::remove_file(&sock_path).with_context(|| format!("whilst deleting {:?}", sock_path))?;

    Ok(())
}
