use std::{collections::HashMap, net::IpAddr};

use anyhow::{Context, Result};
use etherparse::{Icmpv4Slice, Icmpv6Slice, Ipv4Slice, Ipv6Slice, NetSlice, SlicedPacket, TcpSlice, TransportSlice, UdpSlice};
use pcap::{Capture, Device, Inactive};
use tokio::sync::broadcast::Receiver;

use crate::{config::ListenConfig, runtime::{BlockingRunnable, BlockingRunnableBuilder}};

// If we are going to track all traffic between particular IP addresses, the port data will need to be removed.
#[derive(Debug, PartialEq, Eq, Hash)]
struct TcpSession {
  src_ip: IpAddr,
  src_port: u16,
  dst_ip: IpAddr,
  dst_port: u16,
}

struct State {
  seq: u32,
  packet_count: u32,
}

pub struct Builder {
  // cfg: HttpConfig
  iface_name: Option<String>,
}

pub struct Devices {
  cap: Capture<Inactive>,
}

pub fn new(/*cfg: HttpConfig*/) -> Builder {
  Builder{
    iface_name: None,
  }
}

impl Builder {
  pub fn set_interface(mut self, iface_name: String) -> Self {
    self.iface_name = Some(iface_name);
    self
  }
}

impl BlockingRunnableBuilder for Builder {
  fn build(self: Box<Self>) -> Result<Box<dyn crate::runtime::BlockingRunnable + Send>, Box<dyn std::error::Error>> {

    let device = match self.iface_name { // cfg.interfaces.unwrap_or(vec![]).first() {
      Some(iface) => {
        Device::list()?.into_iter().find(|d| d.name == *iface).with_context(|| format!("interface '{}' was not found", iface))?
      },
      None => {
        return Err("no interfaces".into());
      },
    };

    // device
    let cap = Capture::from_device(device)?.promisc(true).timeout(100);

    Ok(Box::new(Devices {
      cap,
    }))
  }
}

impl BlockingRunnable for Devices {
  fn run(self: Box<Self>, cancel_rx: Receiver<()>) -> Result<(), Box< dyn std::error::Error>> {
    let mut cap = self.cap.open()?;
    // let mut cap = Capture::from_device(self.device)?.promisc(true).timeout(100).open()?;

    // sequences
    let mut sequences: HashMap<TcpSession, State> = HashMap::new();

    let (mut packet_count, mut dropped_count, mut if_dropped_count) = (0, 0, 0);

    loop {
      match cap.next_packet() {
        Ok(packet) => {
          match SlicedPacket::from_ethernet(packet.data) {
            Ok(value) => {
              analyze_packet(value, &mut sequences);

            }, // analyze_packet(value),
            Err(err) => println!("Error parsing packet: {:?}", err),
          }
        },
        Err(pcap::Error::TimeoutExpired) => {
          // Just try again on timeout - this makes the program more responsive
          let stats = cap.stats().unwrap();
          if packet_count != stats.received || dropped_count != stats.dropped || if_dropped_count != stats.if_dropped {
            println!("Received: {}, dropped: {}, if_dropped: {}", stats.received, stats.dropped, stats.if_dropped);
          }

          packet_count = stats.received;
          dropped_count = stats.dropped;
          if_dropped_count = stats.if_dropped;
          continue;
        },
        Err(e) => {
          println!("Error: {}", e);
          continue;
        },
      }

      // Check to see if we need to exit
      if !cancel_rx.is_empty() || cancel_rx.is_closed() {
        break;
      }
    }

    Ok(())
  }
}

pub fn listen(cfg: ListenConfig) -> Result<()> {
  let device = match cfg.interfaces.unwrap_or_default().first() {
    Some(iface) => {
      Device::list()?.into_iter().find(|d| d.name == *iface).with_context(|| format!("interface '{}' was not found", iface))?
    },
    None => {
      return Ok(());
    },
  };

  // device

  let mut cap = Capture::from_device(device)?.promisc(true).timeout(100).open()?;

  // match cap.filter("tcp", false) {
  //   Ok(_) => {},
  //   Err(e) => return Err(e.into()),
  // };

  // sequences
  let mut sequences: HashMap<TcpSession, State> = HashMap::new();

  let (mut packet_count, mut dropped_count, mut if_dropped_count) = (0, 0, 0);

  loop {
    match cap.next_packet() {
      Ok(packet) => {
        match SlicedPacket::from_ethernet(packet.data) {
          Ok(value) => {
            analyze_packet(value, &mut sequences);

          }, // analyze_packet(value),
          Err(err) => println!("Error parsing packet: {:?}", err),
        }
      },
      Err(pcap::Error::TimeoutExpired) => {
        // Just try again on timeout - this makes the program more responsive
        let stats = cap.stats().unwrap();
        if packet_count != stats.received || dropped_count != stats.dropped || if_dropped_count != stats.if_dropped {
          println!("Received: {}, dropped: {}, if_dropped: {}", stats.received, stats.dropped, stats.if_dropped);
        }

        packet_count = stats.received;
        dropped_count = stats.dropped;
        if_dropped_count = stats.if_dropped;
        continue;
      },
      Err(e) => {
        println!("Error: {}", e);
        continue;
      },
    }
  }

}

fn process_ipv4_icmpv4(icmpv4_header: &Icmpv4Slice) {
  let icmp_header = icmpv4_header.header();
  println!("ICMPv4, type={:?}", icmp_header.icmp_type);
}

fn process_ipv4_icmpv6() {
  // This should never happen; ICMPv6 should only ever come on IPv6 
  // ref: https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol
  // TODO: render error message
}

fn process_ipv4_tcp(sequences: &mut HashMap<TcpSession, State>, ip_header: &Ipv4Slice, tcp_header: &TcpSlice) {
  let src_port = tcp_header.source_port();
  let dst_port = tcp_header.destination_port();
  let seq = tcp_header.sequence_number();

  let tcp_session = TcpSession {
    src_ip: IpAddr::V4(ip_header.header().source_addr()),
    src_port,
    dst_ip: IpAddr::V4(ip_header.header().destination_addr()),
    dst_port,
  };
        
  match sequences.get_mut(&tcp_session) {
    Some(last_state) => {

      // Sequence is already started
      if seq == last_state.seq {
        println!("= IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}, count={}", tcp_session.src_ip, tcp_session.src_port, tcp_session.dst_ip, tcp_session.dst_port, tcp_header.syn(), tcp_header.ack(), tcp_header.fin(), tcp_header.rst(), ip_header.is_payload_fragmented(), tcp_header.payload().len(), last_state.packet_count);
      } else if seq > last_state.seq {
        println!("> IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}, count={}", tcp_session.src_ip, tcp_session.src_port, tcp_session.dst_ip, tcp_session.dst_port, tcp_header.syn(), tcp_header.ack(), tcp_header.fin(), tcp_header.rst(), ip_header.is_payload_fragmented(), tcp_header.payload().len(), last_state.packet_count);
      } else if seq < last_state.seq {
        println!("Out of order packet!");
      }
      last_state.packet_count += 1;
      last_state.seq = seq;
    },
    None => {
      // New connection
      println!("IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}", tcp_session.src_ip, tcp_session.src_port, tcp_session.dst_ip, tcp_session.dst_port, tcp_header.syn(), tcp_header.ack(), tcp_header.fin(), tcp_header.rst(), ip_header.is_payload_fragmented(), tcp_header.payload().len());
      let s = State {
        packet_count: 1,
        seq,
      };
      sequences.insert(tcp_session, s);
    }
  };

  // sequences.entry(tcp_session).and_modify(|f| { *f = seq }).or_insert_with_key(|_k| {
}

fn process_ipv4_udp(ip_slice: &Ipv4Slice, udp_header: &UdpSlice) {
  let ip_header = ip_slice.header();
  println!("IPv4-UDP [{} -> {}] [{} -> {}] bytes={}", ip_header.source_addr(), ip_header.destination_addr(), udp_header.source_port(), udp_header.destination_port(), udp_header.payload().len());
}

fn process_ipv4_no_transport(_sequences: &mut HashMap<TcpSession, State>, ip_header: &Ipv4Slice) {
  let ip_number = ip_header.payload_ip_number();
  println!("IPv4-no-transport {} {}", ip_number.keyword_str().unwrap_or("---"), ip_number.protocol_str().unwrap_or("unknown"));
  // ip_header.header().
}

fn process_ipv6_icmpv4() {
  // This should never happen; ICMPv6 should only ever come on IPv6 
  // ref: https://en.wikipedia.org/wiki/ICMPv6
  // TODO: render error message
}

fn process_ipv6_icmpv6(icmpv6_header: &Icmpv6Slice) {
  let icmp_header = icmpv6_header.header();
  println!("ICMPv6, type={:?}", icmp_header.icmp_type);
}

fn process_ipv6_tcp(_sequences: &mut HashMap<TcpSession, State>, _ip_header: &Ipv6Slice, tcp_header: &TcpSlice) {
  let _src_port = tcp_header.source_port();
  let _dst_port = tcp_header.destination_port();
  let _seq = tcp_header.sequence_number();
  println!("IPv6-TCP");
}

fn process_ipv6_udp(ip_slice: &Ipv6Slice, udp_header: &UdpSlice) {
  let ip_header = ip_slice.header();
  println!("IPv6-UDP [{} -> {}] [{} -> {}] bytes={}", ip_header.source_addr(), ip_header.destination_addr(), udp_header.source_port(), udp_header.destination_port(), udp_header.payload().len());
}

fn process_ipv6_no_transport(_sequences: &mut HashMap<TcpSession, State>, ip_header: &Ipv6Slice) {
  let ip_number = ip_header .payload().ip_number;
  println!("IPv6-no-transport {} {}", ip_number.keyword_str().unwrap_or("---"), ip_number.protocol_str().unwrap_or("unknown"));
  // ip_header.header().
}


fn analyze_packet(packet: SlicedPacket, sequences: &mut HashMap<TcpSession, State>) {
  match &packet.net {
    Some(NetSlice::Arp(_arp_header)) => { /* Do nothing */ },
    Some(NetSlice::Ipv4(ipv4_header)) => {
      match &packet.transport {
        Some(TransportSlice::Icmpv4(icmpv4_header)) => process_ipv4_icmpv4(icmpv4_header),
        Some(TransportSlice::Icmpv6(_icmpv6_header)) => process_ipv4_icmpv6(),
        Some(TransportSlice::Tcp(tcp_header)) => process_ipv4_tcp(sequences, ipv4_header, tcp_header),
        Some(TransportSlice::Udp(udp_header)) => process_ipv4_udp(ipv4_header, udp_header),
        None => process_ipv4_no_transport(sequences, ipv4_header),
      }
    },
    Some(NetSlice::Ipv6(ipv6_header)) => {
      match &packet.transport {
        Some(TransportSlice::Icmpv4(_icmpv4_header)) => process_ipv6_icmpv4(),
        Some(TransportSlice::Icmpv6(icmpv6_header)) => process_ipv6_icmpv6(icmpv6_header),
        Some(TransportSlice::Tcp(tcp_header)) => process_ipv6_tcp(sequences, ipv6_header, tcp_header),
        Some(TransportSlice::Udp(udp_header)) => process_ipv6_udp(ipv6_header, udp_header),
        None => process_ipv6_no_transport(sequences, ipv6_header),
      }
    },
    None => { /* Do nothing */ }
  }
}

pub fn list() -> Result<()> {
  let list = match Device::list() {
    Ok(x) => x,
    Err(e) => {
      return Err(e.into());
    },
  };

  for d in list.into_iter() {
    println!("{} ({}), addressses {:?}, flags: {:?}", d.name, d.desc.unwrap_or_default(), d.addresses, d.flags)
  }

  Ok(())
}