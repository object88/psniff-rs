use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use log::{error, info};
use pcap::{Capture, Device, Inactive};
use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use crate::{
	config::ListenConfig, runtime::{BlockingRunnable, BlockingRunnableBuilder}, state::{appstate::AppState, interface::Interface}
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

pub enum ReceivedPacketData {
	MovingPacket {
		header: pcap::PacketHeader,
		data: Vec<u8>,
	},

	Counts {
		total: u32,
		os_dropped: u32,
		if_dropped: u32,
	}
}

pub struct Builder {
	iface_name: Option<String>,
	senders: HashMap<Matcher, Sender<ReceivedPacketData>>,
	state: Option<AppState>,
}

pub struct Devices {
	// iface: Arc<Mutex<Interface>>,
	iface: Arc<Interface>,
	cap: Capture<Inactive>,
	senders: HashMap<Matcher, Sender<ReceivedPacketData>>,
}

pub fn new() -> Builder {
	Builder { 
		iface_name: None,
		senders: HashMap::new(),
		state: None,
	}
}

impl Builder {
	pub fn set_interface(mut self, iface_name: String) -> Self {
		self.iface_name = Some(iface_name);
		self
	}

	pub fn set_state(mut self, state: AppState) -> Self {
		self.state = Some(state);
		self
	}

	pub fn set_typed_sender(mut self, m: Matcher, sender: Sender<ReceivedPacketData>) -> Self {
		self.senders.insert(m, sender);
		self
	}
}

impl BlockingRunnableBuilder for Builder {
	fn build(
		self: Box<Self>,
	) -> Result<Box<dyn BlockingRunnable + Send>, Box<dyn std::error::Error>> {
		let (iface_name, device) = match self.iface_name {
			Some(iface_name) => {
				let dev = Device::list()?
					.into_iter()
					.find(|d| d.name == *iface_name)
					.with_context(|| format!("interface '{}' was not found", iface_name))?;
				(iface_name, dev)
			},
			None => {
				return Err("no interfaces".into());
			},
		};

		let iface = Arc::new(Interface::new(iface_name));

		// device
		let cap = Capture::from_device(device)?.promisc(true).timeout(100);

		let state = match self.state {
			Some(x) => x,
			None => {
				return Err("".into());
			}
		};

		// Add the interface to the appstate
		state.interfaces.lock().unwrap().insert(iface.clone());

		Ok(Box::new(Devices {
			iface,
			cap,
			senders: self.senders,
		}))
	}
}

impl BlockingRunnable for Devices {
	fn run(self: Box<Self>, cancel_rx: Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
		let mut cap = self.cap.open()?;

		let (mut packet_count, mut os_dropped_count, mut if_dropped_count) = (0, 0, 0);

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

					let header_clone = *packet.header;
					let data_clone = packet.data.to_vec();
					let p0 = ReceivedPacketData::MovingPacket {
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

					if packet_count == stats.received
						&& os_dropped_count == stats.dropped
						&& if_dropped_count == stats.if_dropped
					{
						continue
					}

					packet_count = stats.received;
					os_dropped_count = stats.dropped;
					if_dropped_count = stats.if_dropped;

					info!(
						"Received: {}, dropped: {}, if_dropped: {}",
						stats.received, stats.dropped, stats.if_dropped
					);

					self.iface.update_counts(packet_count, os_dropped_count, if_dropped_count);

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
