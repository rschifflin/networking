use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use crossbeam::channel;
use mio::{Poll, Token, Waker};
use mio::net::UdpSocket as MioUdpSocket;

use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::daemon::poll;

pub fn handle(msg: FromService,
  poll: &Poll,
  token_map: &mut HashMap<Token, (MioUdpSocket, SocketAddr)>,
  states: &mut HashMap<SocketAddr, State>,
  tx_on_write: &channel::Sender<Token>,
  waker: &Arc<Waker>,
  next_conn_id: &mut usize) {
  match msg {
    FromService::Connect(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, addr)) => {
          respond_tx.send(ToService::IORegistered).ok()
            .and_then(|_| {
              let tx_on_write = tx_on_write.clone();
              let waker = Arc::clone(waker);
              State::init_connect(token, respond_tx, tx_on_write, waker, &conn).ok()
            })
            // would prefer map_none() here if it existed on Option
            .or_else(|| {
              poll::deregister_io(poll, &mut conn);
              None
            })
            .map(|state| {
              token_map.insert(token, (conn, addr));
              states.insert(addr, state)
            });
        }
        None => drop(respond_tx)
      }
    }
    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, addr)) => {
          respond_tx.send(ToService::IORegistered)
            .map_err(|_| poll::deregister_io(poll, &mut conn)).ok()
            .map(|_| {
              let tx_on_write = tx_on_write.clone();
              let waker = Arc::clone(waker);
              let state = State::init_listen(token, respond_tx, tx_on_write, waker);
              token_map.insert(token, (conn, addr));
              states.insert(addr, state)
            });
        },
        None => drop(respond_tx)
      }
    }
  }
}

