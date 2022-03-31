use std::{collections::BTreeSet, fs::File, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use nix::sys::stat::Mode;
use pcap_file::PcapWriter;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    pcap: Option<PathBuf>,
    socket_path: PathBuf,
}

impl Cli {
    fn pcap_writer(&self) -> Result<Option<PcapWriter<File>>> {
        let p = match &self.pcap {
            Some(p) => p,
            None => return Ok(None),
        };
        let f = File::create(p)?;
        println!("opened {:?}", f);
        Ok(Some(PcapWriter::new(f)?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    //console_subscriber::init();
    let cli = Cli::parse();
    let mut pcap_writer = cli.pcap_writer()?;
    let listener = tokio::net::UnixDatagram::bind(&cli.socket_path)?;
    let _unlinker = crabbletalk_afpd::UnlinkOnDrop::new(cli.socket_path);
    let mut clients = BTreeSet::new();
    let mut buf = vec![0u8; 1600];

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = listener.recv_from(&mut buf) => { r }
        }?;
        if n_read < 1 {
            continue;
        }
        let data = &buf[..n_read];
        println!("read {} bytes from {:?}", n_read, addr);
        if let Some(writer) = &mut pcap_writer {
            let now = chrono::offset::Utc::now();
            writer.write(
                now.timestamp() as u32,
                now.timestamp_subsec_nanos(),
                data,
                n_read as u32,
            )?;
        }
        if let Some(path) = addr.as_pathname() {
            if !clients.contains(path) {
                clients.insert(path.to_owned());
            }
            let mut to_remove = BTreeSet::new();
            for client in &clients {
                if client == path {
                    continue;
                }
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

    Ok(())
}
