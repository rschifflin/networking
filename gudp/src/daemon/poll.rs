use std::io;
use std::collections::HashMap;
use std::net::UdpSocket as StdUdpSocket;
use std::net::SocketAddr;

use log::warn;
use mio::{Poll, Token, Interest};
use mio::net::UdpSocket as MioUdpSocket;

use crate::socket::{Socket, PeerType};

pub fn handle_failure(e: io::Error, token_map: &mut HashMap<Token, Socket>) -> io::Error {
  // Call to the system selector failed.
  // We cannot perform any evented IO without it.
  // It's possible this error has non-fatal variants, but it's
  // likely platform-specific. For now we treat them all as fatal.
  let errno = e.raw_os_error();
  for (_, socket) in token_map.into_iter() {
    match &socket.peer_type {
      PeerType::Direct(_addr, state) => {
        let (ref buf_read, ref _buf_write, ref status) = *state.shared;
        let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
        status.set_io_err(errno);
        lock.notify_all();
        drop(lock);
      }

      PeerType::Passive { ref peers, ref listen } => {
        for (_addr, peer_state) in peers.iter() {
          let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
          let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
          status.set_io_err(errno);
          lock.notify_all();
          drop(lock);
        }
      },
    }
  }

  e
}

pub fn register_io(poll: &Poll, io: StdUdpSocket, next_conn_id: &mut usize) -> Option<(Token, MioUdpSocket, SocketAddr)> {
  // Create a mio wrapper for the socket.
  let mut conn = MioUdpSocket::from_std(io);

  // Associate this io with a token
  let token = Token(*next_conn_id);
  *next_conn_id += 1;

  // Register this io with its token for polling
  poll.registry()
    .register(&mut conn, token, Interest::READABLE | Interest::WRITABLE)
    .and_then(|_| conn.local_addr())
    .map(|addr| (token, conn, addr))
    .ok()
}

// If the deregister fails, we'll silently leak the file descriptor
// For now, simply log if this occurs.
// TODO: We could bubble up hanging resources to the main loop,
// where we iterate on trying to deregister them.
pub fn deregister_io(poll: &Poll, io: &mut MioUdpSocket) {
  poll.registry().deregister(io).unwrap_or_else(|e| {
    warn!("Unable to deregister socket from poll on close. The socket fd may leak! Reason: {}", e);
  });
}
