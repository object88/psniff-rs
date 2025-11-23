
use async_trait::async_trait;
use etherparse::SlicedPacket;
use log::error;
use thiserror::Error;
use tokio::sync::broadcast;

use crate::devices::MovingPacket;

// Define a trait that your struct will implement
#[async_trait]
pub trait PacketHandler {
	async fn recv(&mut self) -> Option<MovingPacket>;
	async fn handle_packet(&mut self, value: SlicedPacket<'_>);
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
        let p = pcap::Packet{ header: &x0.header, data: &x0.data };
        match SlicedPacket::from_ethernet(p.data) {
          Ok(value) => {
            handler.handle_packet(value).await;
          },
          Err(err) => {
            error!("Error parsing packet: {:?}", err);
          }
        };
      },
    }
  }
}


#[derive(Debug, Error)]
pub enum BuildError {
	#[error("no receiver")]
	NoReceiver,
}
