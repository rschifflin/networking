use std::collections::hash_map::OccupiedEntry;
use std::sync::Arc;
use std::io;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use crate::types::FromDaemon as ToService;
use crate::state::{State, FSM};
use crate::daemon::poll;

type StateEntry<'a> = OccupiedEntry<'a, Token, (State, MioUdpSocket)>;

pub fn handle(mut entry: StateEntry, poll: &Poll) {
  let (ref mut state, ref mut socket) = entry.get_mut();
  let (ref buf_read, ref buf_write, ref status) = *state.shared;

  // TODO: Should we handle a poisoned lock state here? IE if a thread with a connection panics,
  // what should the daemon do about it? Just close the connection?
  // Likely the client should panic on poison, and the daemon should recover the lock and close the conn on poison
  // For now just panic
  let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

  // NOTE: This status check must be done while holding the readlock to ensure no races occur of the form:
  // time0|thread0: client acquires the readlock.
  // time1|thread0: client observes an open status and no pending reads...
  // time2|thread1: open status changes to closed...
  // time3|thread1: this fn observes the closed status and signals the condvar to wake all _current_ sleepers
  // time4|thread0: ... and then client sleeps, oblivious to the change in status and condvar signal
  // Since after this notify_all, no future notifications are coming, client would sleep forever!
  // The solution is to prevent the client from acquiring the readlock until this fn observes the closed status.
  // That means when the client DOES acquire the readlock, it will observe a closed status and not sleep.
  // Alternatively, if the client acquired the readlock first, it will sleep on the condvar before this fn can notify.
  // Thus ensuring it will hear the notification to wake up and observe the closed status.
  if status.is_closed() {
    poll::close_remote_socket(poll, socket, buf);
    entry.remove();
    return;
  }

  // TODO: Read in loop until we hit WOULDBLOCK
  match socket.recv(&mut state.buf_local) {
    Ok(size) => {
      match &state.fsm {
        FSM::Listen { tx } |
        FSM::Handshaking { tx } => {
          let on_write = |usize| -> io::Result<usize> {
            println!("Called with {}", usize);
            Ok(usize)
          };
          match tx.send(ToService::Connection(Box::new(on_write), Arc::clone(&state.shared))) {
            Ok(_) => {
              buf.push_back(&state.buf_local[..size]).map(|_| buf.notify_one());
              if let FSM::Listen { .. } = state.fsm {
                // TODO: Write this into separate unshared daemon write buffer that doesn't require lock protection
                // What happens if the buffer here is full with user writes? Etc
                let mut buf_w = buf_write.lock().expect("Could not acquire unpoisoned write lock");
                buf_w.push_back(b"hello").expect("Could not write minimal hello! Write buffer too small!");
                drop(buf_w);
              }

              state.fsm = FSM::Connected;
              drop(buf);
            },
            Err(_) => {
              // Failed to create the connection. Deregister
              // NOTE: Setting status is technically not necessary, there is no clientside to observe this
              status.set_client_hup();
              poll::close_remote_socket(poll, socket, buf);
              entry.remove();
            }
          };
        },
        FSM::Connected => {
          buf.push_back(&state.buf_local[..size]).map(|_| buf.notify_one());
          drop(buf);
        }
      }
    },

    Err(e) => {
      if e.kind() == std::io::ErrorKind::WouldBlock {} // This is fine for mio
      else {
        // TODO: Handle errors explicitly. Set remote drop flags based on errorkind
        // Add error flags we can set when we have a semantic error that has no underlying errno code.
        let errno = e.raw_os_error();
        status.set_io_err(errno);
        poll::close_remote_socket(poll, socket, buf);
        entry.remove();
      }
    }
  }
}
