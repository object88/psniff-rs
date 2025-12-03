use async_trait::async_trait;
use etherparse::{NetSlice, SlicedPacket};
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{
	devices::ReceivedPacketData,
	packet_listeners::listener::{self, BuildError, PacketHandler},
	runtime::{Runnable, RunnableBuilder},
};

pub struct ArpListenerBuilder {
	receiver: Option<Receiver<ReceivedPacketData>>,
}

pub fn new() -> ArpListenerBuilder {
	ArpListenerBuilder { receiver: None }
}

impl ArpListenerBuilder {
	pub fn set_receiver(mut self, receiver: Receiver<ReceivedPacketData>) -> Self {
		self.receiver = Some(receiver);
		self
	}
}

pub struct ArpListener {
	receiver: Receiver<ReceivedPacketData>,

	packet_count: u64,
}

#[async_trait]
impl RunnableBuilder for ArpListenerBuilder {
	async fn build(self: Box<Self>) -> Result<Box<dyn Runnable>, Box<dyn std::error::Error>> {
		let receiver = match self.receiver {
			Some(x) => x,
			None => return Err(BuildError::NoReceiver.into()),
		};

		Ok(Box::new(ArpListener {
			receiver,
			packet_count: 0,
		}))
	}
}

#[async_trait]
impl Runnable for ArpListener {
	async fn run(&mut self, cancel_rx: broadcast::Receiver<()>) {
		listener::run(cancel_rx, self).await;
	}
}

#[async_trait]
impl PacketHandler for ArpListener {
	async fn recv(&mut self) -> Option<ReceivedPacketData> {
		self.receiver.recv().await
	}

	async fn handle_packet(&mut self, packet: SlicedPacket<'_>) {
		self.packet_count += 1;

		if let Some(NetSlice::Arp(_arp_header)) = &packet.net {}

		// if let Some(NetSlice::Ipv4(ipv4_header)) = &packet.net && let Some(TransportSlice::Tcp(tcp_header)) = &packet.transport {
		//   process_ipv4_tcp(&mut self.sequences, ipv4_header, tcp_header)
		// }
	}

	async fn handle_packet_count(&mut self, _count: (u64, u64, u64)) {}
}
