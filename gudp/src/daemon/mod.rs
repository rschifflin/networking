use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mio::{Poll, Events, Token, Waker, Interest};
use mio::net::UdpSocket as MioUdpSocket;
use crossbeam::channel::Receiver;

use bring::{Bring, WithOpt};

use crate::constants::{WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};

use crate::types::SharedRingBuf;
use crate::types::ToDaemon as FromService;
use crate::types::FromDaemon as ToService;
use crate::state::State;

pub fn spawn(mut poll: Poll, _waker: Arc<Waker>, rx: Receiver<FromService>) {
  std::thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || {
      let mut events = Events::with_capacity(2); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut states: HashMap<Token, (State, MioUdpSocket)> = HashMap::new();
      // Clear out all msgs from service
      loop {
        for msg in rx.try_iter() {
          match msg {
            FromService::Print(msg) => {
              println!("Got msg: {}", msg);
            },
            FromService::Connect(io, respond_tx) |
            FromService::Listen(io, respond_tx) => {
              println!("Got new socket: {:?}", io);
              let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
              let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
              let buf_read: SharedRingBuf = Arc::new(Mutex::new(Bring::from_vec(buf_read_vec)));
              let buf_write: SharedRingBuf = Arc::new(Mutex::new(Bring::from_vec(buf_write_vec)));

              // Create a mio wrapper for the socket.
              let mut conn = MioUdpSocket::from_std(io);

              // Associate this io with a token
              let token = Token(next_conn_id);
              next_conn_id += 1;

              // Register this io with its token for polling
              poll.registry().register(&mut conn, token, Interest::READABLE | Interest::WRITABLE).expect("Could not register");

              // Simple case: assume all sockets are connected
              // The next step is to encode 'connection' into the state machine;
              //  we will hang onto respond_tx until the state machine sees incoming packets
              //  indicating a virtual connection.
              // Instead for now, we assume an automatic always-connected virtual connection
              // And just immediately send the read/write buffers to the user-facing connection object
              respond_tx.send(
                ToService::Connection(Arc::clone(&buf_read), Arc::clone(&buf_write))
              ).expect("Could not respond with connection state");

              // Create new state machine for the socket. Store the state locally
              // TODO: Initialize it as either the active_open state (connecting) or the passive_open state (listening)
              let state = State::new(buf_read, buf_write);

              // Add to the list
              states.insert(token, (state, conn));
            }
          }
        }

        poll.poll(&mut events, None).expect("Could not poll");

        // Handle reads.
        for event in events.iter() {
          // We can use the token we previously provided to `register` to
          // determine for which type the event is.
          if event.token() != WAKE_TOKEN && event.is_readable() {
            let (ref mut state, ref mut socket) = states.get_mut(&event.token()).expect("Could not look up token");
            {
              let mut buf = state.buf_read.lock().expect("Could not acquire unpoisoned read lock");
              match socket.recv(&mut state.buf_local) {
                Ok(size) => {
                  buf.push_back(&state.buf_local[..size]);
                },
                Err(e) => {
                  if e.kind() == std::io::ErrorKind::WouldBlock {} // This is expected for nonblocking io
                  else {} // Handle bad errors here!
                }
              }
            }
          }
        };

        // Handle writes
        for (_, (ref mut state, ref mut socket)) in states.iter_mut() {
          let mut buf = state.buf_write.lock().expect("Could not acquire unpoisoned write lock");
          let buf = &mut *buf;
          if buf.count() > 0 {
            let buf_local = &mut state.buf_local;
            match buf.with_front(buf_local, |buf_local, bytes| {
              let send = socket.send(&buf_local[..bytes]);
              let opt = match send {
                Ok(_) => WithOpt::Pop,
                Err(_) => WithOpt::Peek
              };
              (send, opt)
            }).expect("Could not pop") {
              Ok(_wrote) => (), // ok
              Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {} // This is expected for nonblocking io
                else {} // Handle bad errors here!
              }
            }
          }
        }
      }
    })
    .expect("Could not spawn daemon");
}

