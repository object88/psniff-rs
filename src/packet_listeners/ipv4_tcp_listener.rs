use async_trait::async_trait;
use etherparse::SlicedPacket;
use log::info;
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{devices::{self, MovingPacket}, packet_listeners::listener::{self, BuildError, PacketHandler}, runtime::{Runnable, RunnableBuilder}};

pub struct Ipv4TcpListenerBuilder {
  receiver: Option<Receiver<devices::MovingPacket>>
}

pub fn new() -> Ipv4TcpListenerBuilder {
  Ipv4TcpListenerBuilder{
    receiver: None,
  }
}

impl Ipv4TcpListenerBuilder {
  pub fn set_receiver(mut self, receiver: Receiver<devices::MovingPacket>) -> Self {
    self.receiver = Some(receiver);
    self
  }
}


pub struct Ipv4TcpListener {
  receiver: Receiver<devices::MovingPacket>,
}

#[async_trait]
impl RunnableBuilder for Ipv4TcpListenerBuilder {
  async fn build(self: Box<Self>) -> Result<Box<dyn Runnable>, Box<dyn std::error::Error>> {
    let receiver = match self.receiver {
      Some(x) => x,
      None => {
        return Err(BuildError::NoReceiver.into())
      },
    };

    Ok(Box::new(Ipv4TcpListener{
      receiver,
    }))
  }
}

#[async_trait]
impl Runnable for Ipv4TcpListener {
	async fn run(&mut self, cancel_rx: broadcast::Receiver<()>) {
    listener::run(cancel_rx, self).await;
  }
}

#[async_trait]
impl PacketHandler for Ipv4TcpListener {
  async fn recv(&mut self) -> Option<MovingPacket> {
    self.receiver.recv().await
  }

	async fn handle_packet(&mut self, value: SlicedPacket<'_>) {
		info!("Processed message {:?}", value);
	}
}
