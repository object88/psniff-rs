use anyhow::{Context, Result};
use clap::{arg, command, Parser};
use etherparse::SlicedPacket;
use pcap::{Capture, Device};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Network interface to capture on
  #[arg(short, long)]
  interface: Option<String>,
}

fn main() -> Result<()> {
  println!("Hello, world!");
  let args = Args::parse();

  let device = match &args.interface {
    Some(iface) => {
      Device::list()?.into_iter().find(|d| d.name == *iface).with_context(|| format!("interface '{}' was not found", iface))?
    },
    None => {
      return Ok(());
    },
  };

  let mut cap = Capture::from_device(device)?.promisc(true).snaplen(6 * 1024).timeout(100).open()?;

  loop {
    match cap.next_packet() {
      Ok(packet) => {
        println!("\n{} bytes", packet.header.len);
                
        match SlicedPacket::from_ethernet(packet.data) {
          Ok(_value) => {}, // analyze_packet(value),
          Err(err) => println!("Error parsing packet: {:?}", err),
        }
      },
      Err(pcap::Error::TimeoutExpired) => {
        // Just try again on timeout - this makes the program more responsive
        continue;
      },
      Err(_) => {
        break;
      },
    }
  }

  Ok(())
}
