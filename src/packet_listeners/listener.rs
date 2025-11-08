use std::{collections::HashMap, net::IpAddr};

use async_trait::async_trait;
use etherparse::{Icmpv4Slice, Icmpv6Slice, Ipv4Slice, Ipv6Slice, NetSlice, SlicedPacket, TcpSlice, TransportSlice, UdpSlice};
use log::{error, info};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{devices, runtime::{Runnable, RunnableBuilder}};

#[derive(Debug, Error)]
pub enum BuildError {
	#[error("no receiver")]
	NoReceiver,
}

pub struct Builder {
  receiver: Option<Receiver<devices::MovingPacket>>
}

pub fn new() -> Builder {
  Builder{
    receiver: None,
  }
}

impl Builder {
  pub fn set_receiver(mut self, receiver: Receiver<devices::MovingPacket>) -> Self {
    self.receiver = Some(receiver);
    self
  }
}

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


pub struct Listener {
  receiver: Receiver<devices::MovingPacket>,

  // sequences
  sequences: HashMap<TcpSession, State>,
}

#[async_trait]
impl RunnableBuilder for Builder {
  async fn build(self: Box<Self>) -> Result<Box<dyn Runnable>, Box<dyn std::error::Error>> {
    let receiver = match self.receiver {
        Some(x) => x,
        None => {
          return Err(BuildError::NoReceiver.into())
        },
    };

    Ok(Box::new(Listener{
      receiver,
      sequences: HashMap::new(),
    }))
  }
}

#[async_trait]
impl Runnable for Listener {
	async fn run(&mut self, mut cancel_rx: broadcast::Receiver<()>) {
    loop {
      tokio::select! {
        _ = cancel_rx.recv() => {
          break;
        },
        x = self.receiver.recv() => {
          let x0 = match x {
            Some(x) => x,
            None => {
              continue;
            }
          };
          let p = pcap::Packet{ header: &x0.header, data: &x0.data };
          match SlicedPacket::from_ethernet(p.data) {
            Ok(value) => {
              analyze_packet(value, &mut self.sequences);
              // info!("received passed packet: {:?}", value);
            },
            Err(err) => {
              error!("Error parsing packet: {:?}", err);
            }
          };
        },
      }
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

fn process_ipv4_tcp(
	sequences: &mut HashMap<TcpSession, State>,
	ip_header: &Ipv4Slice,
	tcp_header: &TcpSlice,
) {
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
				println!(
					"= IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}, count={}",
					tcp_session.src_ip,
					tcp_session.src_port,
					tcp_session.dst_ip,
					tcp_session.dst_port,
					tcp_header.syn(),
					tcp_header.ack(),
					tcp_header.fin(),
					tcp_header.rst(),
					ip_header.is_payload_fragmented(),
					tcp_header.payload().len(),
					last_state.packet_count
				);
			} else if seq > last_state.seq {
				println!(
					"> IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}, count={}",
					tcp_session.src_ip,
					tcp_session.src_port,
					tcp_session.dst_ip,
					tcp_session.dst_port,
					tcp_header.syn(),
					tcp_header.ack(),
					tcp_header.fin(),
					tcp_header.rst(),
					ip_header.is_payload_fragmented(),
					tcp_header.payload().len(),
					last_state.packet_count
				);
			} else if seq < last_state.seq {
				println!("Out of order packet!");
			}
			last_state.packet_count += 1;
			last_state.seq = seq;
		},
		None => {
			// New connection
			println!(
				"IPv4-TCP [{}:{} -> {}:{}] SYN={} ACK={} FIN={} RST={} seq={seq}, frag={}, bytes={}",
				tcp_session.src_ip,
				tcp_session.src_port,
				tcp_session.dst_ip,
				tcp_session.dst_port,
				tcp_header.syn(),
				tcp_header.ack(),
				tcp_header.fin(),
				tcp_header.rst(),
				ip_header.is_payload_fragmented(),
				tcp_header.payload().len()
			);
			let s = State {
				packet_count: 1,
				seq,
			};
			sequences.insert(tcp_session, s);
		},
	};
}

fn process_ipv4_udp(ip_slice: &Ipv4Slice, udp_header: &UdpSlice) {
	let ip_header = ip_slice.header();
	println!(
		"IPv4-UDP [{} -> {}] [{} -> {}] bytes={}",
		ip_header.source_addr(),
		ip_header.destination_addr(),
		udp_header.source_port(),
		udp_header.destination_port(),
		udp_header.payload().len()
	);
}

fn process_ipv4_no_transport(/*_sequences: &mut HashMap<TcpSession, State>,*/ ip_header: &Ipv4Slice) {
	let ip_number = ip_header.payload_ip_number();
	println!(
		"IPv4-no-transport {} {}",
		ip_number.keyword_str().unwrap_or("---"),
		ip_number.protocol_str().unwrap_or("unknown")
	);
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

fn process_ipv6_tcp(
	// _sequences: &mut HashMap<TcpSession, State>,
	_ip_header: &Ipv6Slice,
	tcp_header: &TcpSlice,
) {
	let _src_port = tcp_header.source_port();
	let _dst_port = tcp_header.destination_port();
	let _seq = tcp_header.sequence_number();
	println!("IPv6-TCP");
}

fn process_ipv6_udp(ip_slice: &Ipv6Slice, udp_header: &UdpSlice) {
	let ip_header = ip_slice.header();
	println!(
		"IPv6-UDP [{} -> {}] [{} -> {}] bytes={}",
		ip_header.source_addr(),
		ip_header.destination_addr(),
		udp_header.source_port(),
		udp_header.destination_port(),
		udp_header.payload().len()
	);
}

fn process_ipv6_no_transport(/*_sequences: &mut HashMap<TcpSession, State>,*/ ip_header: &Ipv6Slice) {
	let ip_number = ip_header.payload().ip_number;
	println!(
		"IPv6-no-transport {} {}",
		ip_number.keyword_str().unwrap_or("---"),
		ip_number.protocol_str().unwrap_or("unknown")
	);
	// ip_header.header().
}

fn analyze_packet(packet: SlicedPacket, sequences: &mut HashMap<TcpSession, State>) {
	match &packet.net {
		Some(NetSlice::Arp(_arp_header)) => { /* Do nothing */ },
		Some(NetSlice::Ipv4(ipv4_header)) => match &packet.transport {
			Some(TransportSlice::Icmpv4(icmpv4_header)) => process_ipv4_icmpv4(icmpv4_header),
			Some(TransportSlice::Icmpv6(_icmpv6_header)) => process_ipv4_icmpv6(),
			Some(TransportSlice::Tcp(tcp_header)) => process_ipv4_tcp(sequences, ipv4_header, tcp_header),
			Some(TransportSlice::Udp(udp_header)) => process_ipv4_udp(ipv4_header, udp_header),
			None => process_ipv4_no_transport(/*sequences,*/ ipv4_header),
		},
		Some(NetSlice::Ipv6(ipv6_header)) => match &packet.transport {
			Some(TransportSlice::Icmpv4(_icmpv4_header)) => process_ipv6_icmpv4(),
			Some(TransportSlice::Icmpv6(icmpv6_header)) => process_ipv6_icmpv6(icmpv6_header),
			Some(TransportSlice::Tcp(tcp_header)) => process_ipv6_tcp(/*sequences,*/ ipv6_header, tcp_header),
			Some(TransportSlice::Udp(udp_header)) => process_ipv6_udp(ipv6_header, udp_header),
			None => process_ipv6_no_transport(/*sequences,*/ ipv6_header),
		},
		None => { /* Do nothing */ },
	}
}
