use async_trait::async_trait;
use etherparse::{NetSlice, SlicedPacket};
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{
	devices::ReceivedPacketData, packet_listeners::listener::{self, PacketHandler}, runtime::Runnable, state::appstate::AppState
};

struct GenericListener {
  state: AppState,
  receiver: Receiver<ReceivedPacketData>,
  packet_count: u32,
}

#[async_trait]
impl Runnable for GenericListener {
	async fn run(&mut self, cancel_rx: broadcast::Receiver<()>) {
    listener::run(cancel_rx, self).await;
  }
}

#[async_trait]
impl PacketHandler for GenericListener {
  async fn recv(&mut self) -> Option<ReceivedPacketData> {
    self.receiver.recv().await
  }

	async fn handle_packet(&mut self, packet: SlicedPacket<'_>) {
    self.packet_count += 1;

    if let Some(NetSlice::Arp(_arp_header)) = &packet.net {
      
    }

    // if let Some(NetSlice::Ipv4(ipv4_header)) = &packet.net && let Some(TransportSlice::Tcp(tcp_header)) = &packet.transport {
    //   process_ipv4_tcp(&mut self.sequences, ipv4_header, tcp_header)
    // }
  }

  async fn handle_packet_count(&mut self, _count: (u64, u64, u64)) {

  }
}