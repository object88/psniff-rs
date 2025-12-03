use std::{collections::HashMap, net::IpAddr};

use async_trait::async_trait;
use etherparse::{Ipv4Slice, NetSlice, SlicedPacket, TcpSlice, TransportSlice};
use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{devices::{self, ReceivedPacketData}, packet_listeners::listener::{self, BuildError, PacketHandler}, runtime::{Runnable, RunnableBuilder}};

pub struct Ipv4TcpListenerBuilder {
  receiver: Option<Receiver<devices::ReceivedPacketData>>
}

pub fn new() -> Ipv4TcpListenerBuilder {
  Ipv4TcpListenerBuilder{
    receiver: None,
  }
}

impl Ipv4TcpListenerBuilder {
  pub fn set_receiver(mut self, receiver: Receiver<devices::ReceivedPacketData>) -> Self {
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

pub struct Ipv4TcpListener {
  receiver: Receiver<devices::ReceivedPacketData>,

  packet_count: u64,

  // sequences
  sequences: HashMap<TcpSession, State>,
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
      packet_count: 0,
      sequences: HashMap::new(),
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
  async fn recv(&mut self) -> Option<ReceivedPacketData> {
    self.receiver.recv().await
  }

	async fn handle_packet(&mut self, packet: SlicedPacket<'_>) {
    self.packet_count += 1;

    if let Some(NetSlice::Ipv4(ipv4_header)) = &packet.net && let Some(TransportSlice::Tcp(tcp_header)) = &packet.transport {
      process_ipv4_tcp(&mut self.sequences, ipv4_header, tcp_header)
    }
  }

  async fn handle_packet_count(&mut self, _count: (u64, u64, u64)) {

  }
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

  // info!("Processed message {:?}", packet);
}
