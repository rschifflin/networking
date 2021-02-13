use std::collections::HashMap;
use std::net::UdpSocket as StdUdpSocket;

use mio::{Poll, Token, Interest};
use mio::net::UdpSocket as MioUdpSocket;

use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::state::State;

pub fn handle(msg: FromService, poll: &Poll, states: &mut HashMap<Token, (State, MioUdpSocket)>, next_conn_id: &mut usize) {
  match msg {
    FromService::Print(msg) => println!("Got msg: {}", msg),
      FromService::Connect(io, respond_tx) => {
        println!("Got connect");
        match register_io(poll, io, next_conn_id) {
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
        println!("Got listen");
        match register_io(poll, io, next_conn_id) {
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

// If it fails to register, returns None
fn register_io(poll: &Poll, io: StdUdpSocket, next_conn_id: &mut usize) -> Option<(Token, MioUdpSocket)> {
  println!("Registering new socket: {:?}", io);
  // Create a mio wrapper for the socket.
  let mut conn = MioUdpSocket::from_std(io);

  // Associate this io with a token
  let token = Token(*next_conn_id);
  *next_conn_id += 1;

  // Register this io with its token for polling
  poll.registry()
    .register(&mut conn, token, Interest::READABLE | Interest::WRITABLE)
    .map(|_| (token, conn))
    .ok()
}
