use std::net::UdpSocket as StdUdpSocket;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use mio::{Poll, Events, Token, Interest, Waker};
use mio::net::UdpSocket as MioUdpSocket;
use bring::{Bring, WithOpt};

// Alias for arc of mutex of ring blob
use crate::types::SharedRingBuf;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use crate::Connection;
use crate::state::State;

#[derive(Debug)]
enum ChannelMsg {
  Print(&'static str),
  NewConnection(/* token id*/ usize, /* socket */ StdUdpSocket, /*BufRead*/ SharedRingBuf, /*BufWrite*/ SharedRingBuf)
}

pub struct Service {
  next_conn_id: usize,
  waker: Arc<Waker>,
  daemon_chan: crossbeam::channel::Sender<ChannelMsg>
}

impl Service {
  // Starts the service, spawning the daemon thread and providing access to connections
  pub fn initialize() -> Service {
    let (tx, rx) = crossbeam::channel::unbounded();
    let mut poll = Poll::new().expect("Could not make a poll");
    let mut events = Events::with_capacity(2); // 128 connections ought to be enough for anybody
    const WAKE_TOKEN: Token = Token(0);
    let waker = Arc::new(Waker::new(poll.registry(), WAKE_TOKEN).expect("Could not build new waker"));

    // We need to keep the Waker alive, so we'll create a clone for the
    // thread we create below.
    let daemon_waker = waker.clone();

    // Daemon thread
    std::thread::Builder::new().name("gudp daemon".to_string()).spawn(move || {
      let waker = daemon_waker; // Hold on to the waker here to live as long as this thread
      let mut states: HashMap<Token, (State, MioUdpSocket)> = HashMap::new();

      loop {
        for msg in rx.try_iter() {
          match msg {
            ChannelMsg::Print(msg) => {
              println!("Got msg: {}", msg);
            },
            ChannelMsg::NewConnection(id, io, buf_read, buf_write) => {
              println!("Got connection: {:?}", io);
              // Create new state for the socket. Store the state locally
              let state = State::new(buf_read, buf_write);
              // Create a mio wrapper for the socket.
              let mut conn = MioUdpSocket::from_std(io);
              // Register the mio wrapper with the poll for a token
              let token = Token(id);
              poll.registry().register(&mut conn, token, Interest::READABLE | Interest::WRITABLE).expect("Could not register");
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
    }).expect("Could not spawn daemon");

    Service {
      next_conn_id: 1,
      waker,
      daemon_chan: tx
    }
  }

  // Hands off a UDP socket to return a GUDP connection
  pub fn connect_socket(&mut self, sock: StdUdpSocket) -> Connection {
    let other_sock = sock.try_clone().expect("Could not clone udp socket!");
    let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];

    let buf_read: SharedRingBuf = Arc::new(Mutex::new(Bring::from_vec(buf_read_vec)));
    let buf_write: SharedRingBuf = Arc::new(Mutex::new(Bring::from_vec(buf_write_vec)));
    self.daemon_chan.send(
      ChannelMsg::NewConnection(self.next_conn_id, other_sock, buf_read.clone(), buf_write.clone())
    ).expect("Could not send new socket to gudp thread");
    self.waker.wake().expect("Could not wake");
    self.next_conn_id += 1;

    // Hand off user end of the connection
    Connection::new(self.waker.clone(), buf_read, buf_write)
  }

  pub fn print(&self, msg: &'static str) {
    self.daemon_chan.send(ChannelMsg::Print(msg)).expect("Could not send debug print to gudp thread");
  }
}
