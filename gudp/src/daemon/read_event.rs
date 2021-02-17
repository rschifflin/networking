use std::collections::hash_map::OccupiedEntry;
use std::sync::Arc;
use std::io;
use mio::{Poll, Token};

use crate::socket::{Socket, PeerType};
use crate::types::FromDaemon as ToService;
use crate::state::FSM;
use crate::daemon::poll;
use crate::error;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;

pub fn handle(mut token_entry: TokenEntry, buf_local: &mut [u8], poll: &Poll) {
  let mut socket = token_entry.get_mut();

  // TODO: Read in loop until we hit WOULDBLOCK
  match socket.io.recv_from(buf_local) {
    Err(e) => {
      if e.kind() == std::io::ErrorKind::WouldBlock {} // This is fine for mio, try again later
      else {
        // TODO: Handle errors explicitly. Set remote drop flags based on errorkind
        // Add error flags we can set when we have a semantic error that has no underlying errno code.
        let errno = e.raw_os_error();

        // Iterate thru all peers, removing them
        match socket.peer_type {
          PeerType::Direct(_, ref mut state) => {
            let (ref buf_read, ref _buf_write, ref status) = *state.shared;
            // Must grab lock here first, see note below
            let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
            status.set_io_err(errno);
            buf.notify_all();
            drop(buf);
          },

          PeerType::Passive { ref mut peers, ref mut listen } => {
            for (_addr, peer_state) in peers.iter() {
              let (ref buf_read, ref _buf_write, ref status) = *peer_state.shared;
              let buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");
              status.set_io_err(errno);
              buf.notify_all();
              drop(buf);
            }
          },
        };
        poll::deregister_io(poll, &mut socket.io);
        token_entry.remove();
      }
    },
    Ok((size, peer_addr)) => {
      match socket.peer_type {
        PeerType::Passive { ref mut peers, ref listen } => {
          // if peer_addr is a member of peers
          match (peers.get_mut(&peer_addr), listen) {
            (Some(mut state), _) => { /* handle peer */ },
            (None, Some(listen_opts)) => { /* create new peer */ },
            (None, None) => return, // Discard socket noise
          }
        }

        PeerType::Direct(addr, ref mut state) => {
          if peer_addr != addr { return; } // Discard irrelevant socket noise */
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
            buf.notify_all();
            poll::deregister_io(poll, &mut socket.io);
            drop(buf);
            token_entry.remove();
            return;
          }

          match &state.fsm {
            FSM::Handshaking { token, tx_to_service, tx_on_write, waker } => {
              let on_write = {
                let token = *token;
                let tx_on_write = tx_on_write.clone();
                let waker = Arc::clone(waker);

                move |size| -> io::Result<usize> {
                  tx_on_write.send((token, peer_addr)).map_err(error::cannot_send_to_daemon)?;
                  waker.wake().map_err(error::wake_failed)?;
                  Ok(size)
                }
              };

              match tx_to_service.send(ToService::Connection(Box::new(on_write), Arc::clone(&state.shared))) {
                Ok(_) => {
                  buf.push_back(&mut buf_local[..size]).map(|_| buf.notify_one());
                  state.fsm = FSM::Connected;
                  drop(buf);
                },
                Err(_) => {
                  // Failed to create the connection. Deregister
                  // NOTE: Setting status is technically not necessary, there is no clientside to observe this
                  status.set_client_hup();
                  buf.notify_all();
                  poll::deregister_io(poll, &mut socket.io);
                  drop(buf);
                  token_entry.remove();
                }
              };
            },
            FSM::Connected => {
              buf.push_back(&mut buf_local[..size]).map(|_| buf.notify_one());
              drop(buf);
            }
          }
        }
      }
    }
  }
}
