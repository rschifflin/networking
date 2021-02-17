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
        Some((token, mut conn, _local_addr)) => {
          let tx_on_write = tx_on_write.clone();
          let waker = Arc::clone(waker);
          State::init_connect(token, peer_addr, respond_tx, tx_on_write, waker, &conn).ok()
            .or_else(|| { poll::deregister_io(poll, &mut conn); None })
            .map(|state| {
              let socket = Socket::new(conn, PeerType::Direct(peer_addr, state));
              token_map.insert(token, socket);
            });
        }
        None => drop(respond_tx)
      }
    }

    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, _addr)) => {
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
                Socket::new(conn, PeerType::Passive { peers, listen })
              );
            });
        },
        None => drop(respond_tx)
      }
    }
  }
}
