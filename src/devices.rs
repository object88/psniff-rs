use std::net::Ipv6Addr;

use anyhow::{Context, Result};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap::{Capture, Device, Linktype};

use crate::config::ListenConfig;

pub fn listen(cfg: ListenConfig) -> Result<()> {
  let device = match cfg.interfaces.unwrap_or(vec![]).first() {
    Some(iface) => {
      Device::list()?.into_iter().find(|d| d.name == *iface).with_context(|| format!("interface '{}' was not found", iface))?
    },
    None => {
      return Ok(());
    },
  };

  let mut cap = Capture::from_device(device)?.promisc(true).snaplen(6 * 1024).timeout(100).open()?;

  match cap.filter("tcp", false) {
    Ok(_) => {},
    Err(e) => return Err(e.into()),
  };

  loop {
    match cap.next_packet() {
      Ok(packet) => {
        println!("\n{} bytes", packet.header.len);

        match SlicedPacket::from_ethernet(packet.data) {
          Ok(value) => {
            analyze_packet(value);

          }, // analyze_packet(value),
          Err(err) => println!("Error parsing packet: {:?}", err),
        }
      },
      Err(pcap::Error::TimeoutExpired) => {
        // Just try again on timeout - this makes the program more responsive
        continue;
      },
      Err(_) => {
        continue;
      },
    }
  }

}

fn analyze_packet(packet: SlicedPacket) {
  // Analyze link layer
  // if let Some(link) = &packet.link {
  //   println!("Link layer: {:?}", link);
  // }

  // Analyze network layer
  match &packet.net {
    Some(NetSlice::Arp(arp)) => {
    },
    Some(NetSlice::Ipv4(ipv4)) => {
      let source = ipv4.header().source_addr();
      let dest = ipv4.header().destination_addr();
      
      println!("IPv4: {} -> {}", source, dest);
      println!("Protocol: {:?}", ipv4.header().protocol());
    },
    Some(NetSlice::Ipv6(ipv6)) => {
      let source = Ipv6Addr::from(ipv6.header().source_addr());
      let dest = Ipv6Addr::from(ipv6.header().destination_addr());
      
      println!("IPv6: {} -> {}", source, dest);
      // println!("Next Header: {}", ipv6.next_header());
    },
    None => println!("No IP layer found"),
  }

  // Analyze transport layer
  match &packet.transport {
    Some(TransportSlice::Tcp(tcp)) => {
      println!("TCP: Port {} -> {}", tcp.source_port(), tcp.destination_port());
      println!("Flags: SYN={} ACK={} FIN={} RST={}", tcp.syn(), tcp.ack(), tcp.fin(), tcp.rst());
      println!("Sequence: {}, Window: {}", tcp.sequence_number(), tcp.window_size());
    }
    Some(TransportSlice::Udp(udp)) => {
      println!("UDP: Port {} -> {}", udp.source_port(), udp.destination_port());
      println!("Length: {}", udp.length());
    }
    Some(TransportSlice::Icmpv4(_)) => {
      println!("ICMPv4 packet");
    }
    Some(TransportSlice::Icmpv6(_)) => {
      println!("ICMPv6 packet");
    }
    None => println!("No transport layer found"),
  }

  // // Analyze payload if present
  // let payload = &packet.payload;
  // if !payload.is_empty() {
  //   println!("Payload: {} bytes", payload.len());
    
  //   // Print the first few bytes of the payload
  //   let preview_len = std::cmp::min(16, payload.len());
  //   print!("Preview: ");
  //   for byte in &payload[0..preview_len] {
  //     print!("{:02x} ", byte);
  //   }
  //   println!();
  // } else {
  //   println!("Payload: empty");
  // }
}

pub fn list() -> Result<()> {
  let list = match Device::list() {
    Ok(x) => x,
    Err(e) => {
      return Err(e.into());
    },
  };

  for d in list.into_iter() {
    print!("{} ({}), addressses {:?}, flags: {:?}\n", d.name, d.desc.unwrap_or_default(), d.addresses, d.flags)
  }

  Ok(())
}