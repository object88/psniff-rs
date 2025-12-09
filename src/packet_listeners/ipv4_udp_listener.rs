use async_trait::async_trait;
use etherparse::{Ipv4Slice, NetSlice, SlicedPacket, TransportSlice, UdpSlice};
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{
	devices::{self, ReceivedPacketData},
	packet_listeners::listener::{self, BuildError, PacketHandler},
	runtime::{Runnable, RunnableBuilder},
};

pub struct Ipv4UdpListenerBuilder {
	receiver: Option<Receiver<devices::ReceivedPacketData>>,
}

pub fn new() -> Ipv4UdpListenerBuilder {
	Ipv4UdpListenerBuilder { receiver: None }
}

impl Ipv4UdpListenerBuilder {
	pub fn set_receiver(mut self, receiver: Receiver<devices::ReceivedPacketData>) -> Self {
		self.receiver = Some(receiver);
		self
	}
}

pub struct Ipv4UdpListener {
	receiver: Receiver<devices::ReceivedPacketData>,
}

#[async_trait]
impl RunnableBuilder for Ipv4UdpListenerBuilder {
	async fn build(self: Box<Self>) -> Result<Box<dyn Runnable>, Box<dyn std::error::Error>> {
		let receiver = match self.receiver {
			Some(x) => x,
			None => return Err(BuildError::NoReceiver.into()),
		};

		Ok(Box::new(Ipv4UdpListener { receiver }))
	}
}

#[async_trait]
impl Runnable for Ipv4UdpListener {
	async fn run(&mut self, cancel_rx: broadcast::Receiver<()>) {
		listener::run(cancel_rx, self).await
	}
}

#[async_trait]
impl PacketHandler for Ipv4UdpListener {
	async fn recv(&mut self) -> Option<ReceivedPacketData> {
		self.receiver.recv().await
	}

	async fn handle_packet(&mut self, packet: SlicedPacket<'_>) {
		if let Some(NetSlice::Ipv4(ipv4_header)) = &packet.net
			&& let Some(TransportSlice::Udp(udp_header)) = &packet.transport
		{
			process_ipv4_udp(ipv4_header, udp_header)
		}
	}

	async fn handle_packet_count(&mut self, _count: (u64, u64, u64)) {}
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
