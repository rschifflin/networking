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
        Some((token, conn)) => {
          respond_tx.send(ToService::IORegistered).map(|_| {
            State::init_connect(respond_tx).map(|state| {
              states.insert(token, (state, conn));
            });
          }).ok(); // If this result is an Err, we handle it by simply not registering
        }
        None => drop(respond_tx)
      }
    }
    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, conn)) => {
          respond_tx.send(ToService::IORegistered).map(|_| {
            let state = State::init_listen(respond_tx);
            states.insert(token, (state, conn));
          }).ok(); // If this result is an Err, we handle it by simply not registering
        },
        None => drop(respond_tx)
      }
    }
  }
}

