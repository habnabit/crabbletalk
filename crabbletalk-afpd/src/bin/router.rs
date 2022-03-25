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
    #[clap(short = 'f', long)]
    mkfifo: bool,
}

impl Cli {
    fn pcap_writer(&self) -> Result<Option<PcapWriter<File>>> {
        let p = match &self.pcap {
            Some(p) => p,
            None => return Ok(None),
        };
        let f = if self.mkfifo {
            nix::unistd::mkfifo(p, Self::fifo_mode())?;
            std::fs::OpenOptions::new().write(true).open(p)?
        } else {
            File::create(p)?
        };
        Ok(Some(PcapWriter::new(f)?))
    }

    fn fifo_mode() -> Mode {
        Mode::S_IRUSR
            | Mode::S_IWUSR
            | Mode::S_IRGRP
            | Mode::S_IWGRP
            | Mode::S_IROTH
            | Mode::S_IWOTH
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let listener = tokio::net::UnixDatagram::bind(&cli.socket_path)?;
    let mut clients = BTreeSet::new();
    let mut buf = vec![0u8; 1600];
    let mut pcap_writer = cli.pcap_writer()?;

    loop {
        let (n_read, addr) = tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            r = listener.recv_from(&mut buf) => { r }
        }?;
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

    std::fs::remove_file(&cli.socket_path)
        .with_context(|| format!("whilst deleting {:?}", &cli.socket_path))?;

    Ok(())
}
