use async_trait::async_trait;
use etherparse::SlicedPacket;
use log::error;
use thiserror::Error;
use tokio::sync::broadcast;

use crate::devices::ReceivedPacketData;

// Define a trait that your struct will implement
#[async_trait]
pub trait PacketHandler {
	async fn recv(&mut self) -> Option<ReceivedPacketData>;
	async fn handle_packet(&mut self, value: SlicedPacket<'_>);
	async fn handle_packet_count(&mut self, value: (u64, u64, u64));
}

pub async fn run<T: PacketHandler>(mut cancel_rx: broadcast::Receiver<()>, handler: &mut T) {
	loop {
		tokio::select! {
			_ = cancel_rx.recv() => {
				break;
			},
			x = handler.recv() => {
				let x0 = match x {
					Some(x) => x,
					None => {
						continue;
					}
				};
				match x0 {
					ReceivedPacketData::MovingPacket { header, data } => {
						// let p = pcap::Packet{ &header, &data };
						match SlicedPacket::from_ethernet(&data) {
							Ok(value) => {
								handler.handle_packet(value).await;
							},
							Err(err) => {
								error!("Error parsing packet: {:?}", err);
							}
						};
					},
					ReceivedPacketData::Counts { total, os_dropped, if_dropped } => {

					},
				}
			},
		}
	}
}

#[derive(Debug, Error)]
pub enum BuildError {
	#[error("no receiver")]
	NoReceiver,
}
