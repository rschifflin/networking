use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;

use crossbeam::channel;

use mio::net::UdpSocket as MioUdpSocket;
use mio::{Waker, Token};

use crate::types::FromDaemon as ToService;
use crate::state::State;

pub struct Socket {
  pub io: MioUdpSocket,
  pub peer_type: PeerType
}

pub struct ListenOpts {
  pub token: Token,
  pub tx_to_service: channel::Sender<ToService>,
  pub tx_on_write: channel::Sender<(Token, SocketAddr)>,
  pub waker: Arc<Waker>
}

impl ListenOpts {
  pub fn new(
    token: Token,
    tx_to_service: channel::Sender<ToService>,
    tx_on_write: channel::Sender<(Token, SocketAddr)>,
    waker: Arc<Waker>) -> ListenOpts {
      ListenOpts { token, tx_to_service, tx_on_write, waker }
  }
}

impl Socket {
  pub fn new(io: MioUdpSocket, peer_type: PeerType) -> Socket {
    Socket { io, peer_type }
  }
}

pub enum PeerType {
  Direct(SocketAddr, State),
  Passive {
    peers: HashMap<SocketAddr, State>,
    listen: Option<ListenOpts>
  }
}
