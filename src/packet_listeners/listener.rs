use async_trait::async_trait;
use etherparse::SlicedPacket;
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

pub struct Listener {
  receiver: Receiver<devices::MovingPacket>
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
              info!("received passed packet: {:?}", value);
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