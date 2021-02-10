use std::collections::HashMap;
use std::sync::{Arc, Mutex, Condvar};

use mio::{Poll, Events, Token, Waker, Interest};
use mio::net::UdpSocket as MioUdpSocket;
use crossbeam::channel::Receiver;

use bring::{Bring, WithOpt};

use crate::constants::{WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;
use crate::types::FromDaemon as ToService;
use crate::state::{State, FSM};

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
          let is_listening = if let FromService::Listen(..) = msg { true } else { false };
          match msg {
            FromService::Print(msg) => {
              println!("Got msg: {}", msg);
            },
            FromService::Connect(io, respond_tx) |
            FromService::Listen(io, respond_tx) => {
              println!("Got new socket: {:?}", io);
              let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
              let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
              let buf_read = Mutex::new(Bring::from_vec(buf_read_vec));
              let buf_write = Mutex::new(Bring::from_vec(buf_write_vec));
              let read_cond = Condvar::new();
              let shared_state = Arc::new((buf_read, buf_write, read_cond));

              // Create a mio wrapper for the socket.
              let mut conn = MioUdpSocket::from_std(io);

              // Associate this io with a token
              let token = Token(next_conn_id);
              next_conn_id += 1;

              // Register this io with its token for polling
              poll.registry().register(&mut conn, token, Interest::READABLE | Interest::WRITABLE).expect("Could not register");

              // Create new state machine for the socket. Store the state locally
              let state = if is_listening {
                State::init_listen(respond_tx, shared_state)
              } else {
                respond_tx.send(
                  ToService::Connection(Arc::clone(&shared_state))
                ).expect("Could not respond with connection state");

                State::init_connect(shared_state)
              };

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
            let (ref mut state, ref socket) = states.get_mut(&event.token()).expect("Could not look up token");
            {
              let (ref buf_read, ref _buf_write, ref read_cond) = *state.shared;
              let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
              match socket.recv(&mut state.buf_local) {
                Ok(size) => {
                  //NOTE: The consensus _seems_ to be that we should notify the condvar while still holding the lock
                  buf.push_back(&state.buf_local[..size]).map(|_| read_cond.notify_one());
                  drop(buf);

                  // If we were listening, we now know we have a live connection to accept
                  if let FSM::Listen{ tx } = &state.fsm {
                    tx.send(
                      ToService::Connection(Arc::clone(&state.shared))
                    ).expect("Could not finish listening with connection state");
                    state.fsm = FSM::Connected;
                  }
                },
                Err(e) => {
                  drop(buf);
                  if e.kind() == std::io::ErrorKind::WouldBlock {} // This is expected for nonblocking io
                  else {} // Handle bad errors here!
                }
              }
            }
          }
        };

        // Handle writes
        for (_, (ref mut state, ref socket)) in states.iter_mut() {
          let (ref _buf_read, ref buf_write, ref _read_cond) = *state.shared;
          let mut buf = buf_write.lock().expect("Could not acquire unpoisoned write lock");
          let buf = &mut *buf;
          let buf_local = &mut state.buf_local;
          match buf.with_front(buf_local, |buf_local, bytes| {
            let send = socket.send(&buf_local[..bytes]);
            let opt = match send {
              Ok(_) => WithOpt::Pop,
              Err(_) => WithOpt::Peek
            };
            (send, opt)
          }) {
            None => (), // Nothing was on the ring or our buf was too small. Simply no-op the write
            Some(Ok(_wrote)) => (), // There was data on the buffer and we were able to pop it and send it!
            Some(Err(e)) => {
              if e.kind() == std::io::ErrorKind::WouldBlock {} // There was data on the buffer but we would've blocked if we tried to send it, so we left it alone
              else {} // There was data on the buffer but the socket errored when we tried to send it! We should close the resource here
            }
          }
        }
      }
    })
    .expect("Could not spawn daemon");
}

