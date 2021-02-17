use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use crossbeam::channel;
use mio::{Poll, Token, Waker};

use crate::socket::{Socket, PeerType, ListenOpts};
use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::daemon::poll;

pub fn handle(msg: FromService,
  poll: &Poll,
  token_map: &mut HashMap<Token, Socket>,
  tx_on_write: &channel::Sender<(Token, SocketAddr)>,
  waker: &Arc<Waker>,
  next_conn_id: &mut usize) {
  match msg {
    FromService::Connect(io, respond_tx, peer_addr) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, local_addr)) => {
          let tx_on_write = tx_on_write.clone();
          let waker = Arc::clone(waker);
          let state = State::init_connect(token, respond_tx, tx_on_write, waker);
          conn.send_to(b"hello", peer_addr).ok()
            .or_else(|| { poll::deregister_io(poll, &mut conn); None })
            .map(|_| {
              let socket = Socket::new(conn, local_addr, PeerType::Direct(peer_addr, state));
              token_map.insert(token, socket);
            });
        }
        None => drop(respond_tx)
      }
    }

    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, local_addr)) => {
          // TODO: Build callback for listener with mpsc for listen_closed_events
          respond_tx.send(ToService::Listener)
            .map_err(|_| poll::deregister_io(poll, &mut conn)).ok()
            .map(|_| {
              let tx_on_write = tx_on_write.clone();
              let waker = Arc::clone(waker);
              let peers = HashMap::new();
              let listen = Some(ListenOpts::new(token, respond_tx, tx_on_write, waker));
              token_map.insert(
                token,
                Socket::new(conn, local_addr, PeerType::Passive { peers, listen })
              );
            });
        },
        None => drop(respond_tx)
      }
    }
  }
}
