// use std::net::SocketAddr;

use mio::net::UdpSocket as MioUdpSocket;

use crate::state::State;

pub struct Socket {
  pub io: MioUdpSocket,
  pub peer_type: PeerType
}

impl Socket {
  pub fn new(io: MioUdpSocket, peer_type: PeerType) -> Socket {
    Socket { io, peer_type }
  }
}

pub enum PeerType {
  Direct(/*TODO: SocketAddr,*/State)
}
