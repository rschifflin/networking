use std::collections::HashMap;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::daemon::poll;

pub fn handle(msg: FromService, poll: &Poll, states: &mut HashMap<Token, (State, MioUdpSocket)>, next_conn_id: &mut usize) {
  match msg {
    FromService::Connect(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn)) => {
          respond_tx.send(ToService::IORegistered).ok()
            .and_then(|_| State::init_connect(respond_tx, &conn).ok())
            // would prefer map_none() here if it existed on Option
            .or_else(|| {
              poll::deregister_io(poll, &mut conn);
              None
            })
            .map(|state| states.insert(token, (state, conn)));
        }
        None => drop(respond_tx)
      }
    }
    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn)) => {
          respond_tx.send(ToService::IORegistered)
            .map_err(|_| poll::deregister_io(poll, &mut conn)).ok()
            .map(|_| {
              let state = State::init_listen(respond_tx);
              states.insert(token, (state, conn));
            });
        },
        None => drop(respond_tx)
      }
    }
  }
}

