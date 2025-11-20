use std::collections::HashMap;

use anyhow::{Context, Result};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use log::{error, info};
use pcap::{Capture, Device, Inactive};
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{
	config::ListenConfig,
	runtime::{BlockingRunnable, BlockingRunnableBuilder},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Matcher {
	Arp,
	IPv4_ICMPv4,
	IPv4_TCP,
	IPv4_UDP,
	IPv6_ICMPv6,
	IPv6_TCP,
	IPv6_UDP,
	Missing,
	Unexpected,
}

pub struct MovingPacket {
	pub header: pcap::PacketHeader,
	pub data: Vec<u8>,
}

pub struct Builder {
	// cfg: HttpConfig
	iface_name: Option<String>,
	senders: HashMap<Matcher, Sender<MovingPacket>>,
}

pub struct Devices {
	cap: Capture<Inactive>,
	senders: HashMap<Matcher, Sender<MovingPacket>>,
}

pub fn new<'a>(/*cfg: HttpConfig*/) -> Builder {
	Builder { 
		iface_name: None,
		senders: HashMap::new(),
	}
}

impl Builder {
	pub fn set_interface(mut self, iface_name: String) -> Self {
		self.iface_name = Some(iface_name);
		self
	}

	// pub fn set_typed_sender(mut self, m: Matcher, sender: Sender<pcap::Packet<'static>>) -> Self {
	pub fn set_typed_sender(mut self, m: Matcher, sender: Sender<MovingPacket>) -> Self {
		self.senders.insert(m, sender);
		self
	}
}

impl BlockingRunnableBuilder for Builder {
	fn build(
		self: Box<Self>,
	) -> Result<Box<dyn crate::runtime::BlockingRunnable + Send>, Box<dyn std::error::Error>> {
		let device = match self.iface_name {
			// cfg.interfaces.unwrap_or(vec![]).first() {
			Some(iface) => Device::list()?
				.into_iter()
				.find(|d| d.name == *iface)
				.with_context(|| format!("interface '{}' was not found", iface))?,
			None => {
				return Err("no interfaces".into());
			},
		};

		// device
		let cap = Capture::from_device(device)?.promisc(true).timeout(100);

		Ok(Box::new(Devices {
			cap,
			senders: self.senders,
		}))
	}
}

impl BlockingRunnable for Devices {
	fn run(self: Box<Self>, cancel_rx: Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
		let mut cap = self.cap.open()?;

		// sequences
		// let mut sequences: HashMap<TcpSession, State> = HashMap::new();

		let (mut packet_count, mut dropped_count, mut if_dropped_count) = (0, 0, 0);

		loop {
			// Check to see if we need to exit
			if !cancel_rx.is_empty() || cancel_rx.is_closed() {
				break;
			}

			match cap.next_packet() {
				Ok(packet) => {
					let m = match SlicedPacket::from_ethernet(packet.data) {
						Ok(sliced_packet) => {
							match &sliced_packet.net {
								Some(NetSlice::Arp(_)) => Matcher::Arp,
								Some(NetSlice::Ipv4(ipv4_header)) => match &sliced_packet.transport {
									Some(TransportSlice::Icmpv4(_)) => Matcher::IPv4_ICMPv4,
									Some(TransportSlice::Icmpv6(_)) => Matcher::Unexpected,
									Some(TransportSlice::Tcp(_)) => Matcher::IPv4_TCP,
									Some(TransportSlice::Udp(_)) => Matcher::IPv4_UDP,
									None => {
										let ip_number = ipv4_header.payload_ip_number();
										info!(
											"IPv4-no-transport {} {}",
											ip_number.keyword_str().unwrap_or("---"),
											ip_number.protocol_str().unwrap_or("unknown")
										);
										continue
									}
								},
								Some(NetSlice::Ipv6(ipv6_header)) => match &sliced_packet.transport {
									Some(TransportSlice::Icmpv4(_)) => Matcher::Unexpected,
									Some(TransportSlice::Icmpv6(_)) => Matcher::IPv6_ICMPv6,
									Some(TransportSlice::Tcp(_)) => Matcher::IPv6_TCP,
									Some(TransportSlice::Udp(_)) => Matcher::IPv6_UDP,
									None => {
										let ip_number = ipv6_header.payload().ip_number;
										info!(
											"IPv6-no-transport {} {}",
											ip_number.keyword_str().unwrap_or("---"),
											ip_number.protocol_str().unwrap_or("unknown")
										);
										continue
									}
								},
								None => Matcher::Missing,
							}
						},
						Err(err) => {
							error!("Error parsing packet: {:?}", err);
							continue;
						}
					};

					let header_clone = packet.header.clone();
					let data_clone = packet.data.to_vec();
					let p0 = MovingPacket {
						header: header_clone,
						data: data_clone,
					};
					let s = match self.senders.get(&m) {
						Some(x) => x,
						None => continue,
					};
					let _ = s.blocking_send(p0);
				},
				Err(pcap::Error::TimeoutExpired) => {
					// Just try again on timeout - this makes the program more responsive
					let stats = cap.stats().unwrap();
					if packet_count != stats.received
						|| dropped_count != stats.dropped
						|| if_dropped_count != stats.if_dropped
					{
						println!(
							"Received: {}, dropped: {}, if_dropped: {}",
							stats.received, stats.dropped, stats.if_dropped
						);
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

		Ok(())
	}
}

pub fn listen(cfg: ListenConfig) -> Result<()> {
	let device = match cfg.interfaces.unwrap_or_default().first() {
		Some(iface) => Device::list()?
			.into_iter()
			.find(|d| d.name == *iface)
			.with_context(|| format!("interface '{}' was not found", iface))?,
		None => {
			return Ok(());
		},
	};

	let mut cap = Capture::from_device(device)?
		.promisc(true)
		.timeout(100)
		.open()?;

	// match cap.filter("tcp", false) {
	//   Ok(_) => {},
	//   Err(e) => return Err(e.into()),
	// };

	let (mut packet_count, mut dropped_count, mut if_dropped_count) = (0, 0, 0);

	loop {
		match cap.next_packet() {
			Ok(packet) => {
				match SlicedPacket::from_ethernet(packet.data) {
					Ok(_value) => {
						// analyze_packet(value/*, &mut sequences */);
						todo!();
					}, // analyze_packet(value),
					Err(err) => println!("Error parsing packet: {:?}", err),
				}
			},
			Err(pcap::Error::TimeoutExpired) => {
				// Just try again on timeout - this makes the program more responsive
				let stats = cap.stats().unwrap();
				if packet_count != stats.received
					|| dropped_count != stats.dropped
					|| if_dropped_count != stats.if_dropped
				{
					println!(
						"Received: {}, dropped: {}, if_dropped: {}",
						stats.received, stats.dropped, stats.if_dropped
					);
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

pub fn list() -> Result<()> {
	let list = match Device::list() {
		Ok(x) => x,
		Err(e) => {
			return Err(e.into());
		},
	};

	for d in list.into_iter() {
		println!(
			"{} ({}), addressses {:?}, flags: {:?}",
			d.name,
			d.desc.unwrap_or_default(),
			d.addresses,
			d.flags
		)
	}

	Ok(())
}
