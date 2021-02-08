use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use blob_ring::{BlobRing, WithOpt};

// Alias for arc of mutex of ring blob
use crate::types::SharedRingBuf;
use crate::constants::CONFIG_BUF_SIZE_BYTES;
use crate::Connection;
use crate::state::State;

#[derive(Debug)]
enum ChannelMsg {
  Print(&'static str),
  NewConnection(UdpSocket, /*BufRead*/ SharedRingBuf, /*BufWrite*/ SharedRingBuf)
}

pub struct Service {
  daemon_chan: crossbeam::channel::Sender<ChannelMsg>
}

impl Service {
  // Starts the service, spawning the daemon thread and providing access to connections
  pub fn initialize() -> Service {
    let (tx, rx) = crossbeam::channel::unbounded();

    // Daemon thread
    std::thread::Builder::new().name("gudp daemon".to_string()).spawn(move || {
      let mut states: Vec<(State, UdpSocket)> = vec![];
      loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        match rx.try_recv() {
          Ok(ChannelMsg::Print(msg)) => {
            println!("Got msg: {}", msg);
          },
          Ok(ChannelMsg::NewConnection(io, buf_read, buf_write)) => {
            // Create new state for the socket. Store the state locally
            // TODO: Hash by port pairs or something instead for easy removal
            println!("Got connection: {:?}", io);
            states.push((State::new(buf_read, buf_write), io));
          },
          _ => {}
        };

        for (state, socket) in states.iter_mut() {
          // Read socket into bufread
          {
            let mut buf = state.buf_read.lock().expect("Could not acquire unpoisoned read lock");
            let recv = socket.recv(&mut state.buf_local);
            if let Ok(size) = recv {
              buf.push_blob_back(&state.buf_local[..size]);
            }
          }

          // Write bufwrite into socket
          {
            let mut buf = state.buf_write.lock().expect("Could not acquire unpoisoned write lock");
            let buf = &mut *buf;
            if buf.count() > 0 {
              let buf_local = &mut state.buf_local;
              buf.with_blob_front(buf_local, |buf_local, bytes| {
                let send = socket.send(&buf_local[..bytes]);
                let opt = match send { Ok(_) => WithOpt::Pop, Err(_) => WithOpt::Peek };
                (send, opt)
              }).expect("Could not pop");
            }
          }
        }
      }
    }).expect("Could not spawn daemon");

    Service {
      daemon_chan: tx
    }
  }

  // Hands off a UDP socket to return a GUDP connection
  pub fn connect_socket(&self, sock: UdpSocket) -> Connection {
    let other_sock = sock.try_clone().expect("Could not clone udp socket!");
    let buf_read_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];
    let buf_write_vec = vec![0u8; CONFIG_BUF_SIZE_BYTES];

    let buf_read: SharedRingBuf = Arc::new(Mutex::new(BlobRing::from_vec(buf_read_vec)));
    let buf_write: SharedRingBuf = Arc::new(Mutex::new(BlobRing::from_vec(buf_write_vec)));
    self.daemon_chan.send(ChannelMsg::NewConnection(other_sock, buf_read.clone(), buf_write.clone())).expect("Could not send new socket to gudp thread");

    // Send a Wake to the daemon to acknowledge this new connection

    Connection::new(sock, buf_read, buf_write)
  }

  pub fn print(&self, msg: &'static str) {
    self.daemon_chan.send(ChannelMsg::Print(msg)).expect("Could not send debug print to gudp thread");
  }
}
