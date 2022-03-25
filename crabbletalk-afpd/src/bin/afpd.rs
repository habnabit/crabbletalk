use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use crabbletalk::link::decode_appletalk;
use pnet::packet::Packet;

fn pcap_path() -> PathBuf {
    let now = chrono::offset::Utc::now();
    format!("atalk-{}.pcap", now.format("%Y%m%d_%H%M%S")).into()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let sock_path: PathBuf = args[1].parse()?;
    let router_path: PathBuf = args[2].parse()?;
    let pcap_path = pcap_path();
    let sock = tokio::net::UnixDatagram::bind(&sock_path)?;
    sock.connect(&router_path)?;
    sock.send(b"").await?;
    let mut pcap_writer = pcap_file::PcapWriter::new(std::fs::File::create(&pcap_path)?)?;
    let mut buf = vec![0u8; 1600];

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = sock.recv_from(&mut buf) => { r }
        }?;
        let now = chrono::offset::Utc::now();
        let data = &buf[..n_read];
        if let Some(p) = decode_appletalk(data) {
            println!("read {} bytes: {:#?}\n  {:?}", n_read, p, p.payload());
        }
        pcap_writer.write(
            now.timestamp() as u32,
            now.timestamp_subsec_nanos(),
            data,
            n_read as u32,
        )?;
    }

    std::fs::remove_file(&sock_path).with_context(|| format!("whilst deleting {:?}", sock_path))?;

    Ok(())
}
