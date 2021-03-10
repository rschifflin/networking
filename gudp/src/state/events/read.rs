use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use crate::types::FromDaemon as ToService;
use crate::error;
use crate::state::{State, FSM, Deps};
use crate::timer::{Timers, TimerKind};
use crate::constants::{header, time_ms};

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection is terminal and can be cleaned up
  pub fn read<D: Deps>(&mut self, local_addr: SocketAddr, peer_addr: SocketAddr, size: usize, deps: &mut D) -> bool {
    let when = deps.now();
    let timers = deps.timers();
    timers.remove((self.socket_id, TimerKind::Timeout), self.last_recv + time_ms::TIMEOUT);
    self.last_recv = when;
    timers.add((self.socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);

    let (ref buf_read, ref buf_write, ref status) = *self.shared;

    // TODO: Should we handle a poisoned lock state here? IE if a thread with a connection panics,
    // what should the daemon do about it? Just close the connection?
    // Likely the client should panic on poison, and the daemon should recover the lock and close the conn on poison
    // For now just panic
    let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

    // The connection only sets app_has_hup on drop, which can only occur
    // when all clones have been dropped (they are simply behind an arc).
    // Thus, we can guarantee there are no condvar-listeners to notify
    if status.app_has_hup() {
      let buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
      if buf_write.count() > 0 {
        return true
      } else {
        self.clear_timers(deps.timers());
        return false
      }
    }

    // App has not hung up
    match &mut self.fsm {
      FSM::Handshaking { conn_opts } => {
        let on_write = {
          let token = conn_opts.token;
          let tx_on_write = conn_opts.tx_on_write.clone();
          let waker = Arc::clone(&conn_opts.waker);

          move |size| -> io::Result<usize> {
            tx_on_write.send((token, peer_addr)).map_err(error::cannot_send_to_daemon)?;
            waker.wake().map_err(error::wake_failed)?;
            Ok(size)
          }
        };

        match conn_opts.tx_to_service.send(ToService::Connection(Arc::new(on_write), Arc::clone(&self.shared), (local_addr, peer_addr))) {
          Ok(_) => {
            // Discard heartbeats (aka empty payloads)
            // TODO: Do we ack if we can't push back?
            if size > header::SIZE_BYTES {
              // TODO: Warn if fails from src buffer too small or dst buffer full?
              buf.push_back(&mut deps.buffer(header::SIZE_BYTES..size)).map(|_| {
                buf.notify_one();
              });
              ack((self.local_addr, peer_addr), deps);
            }

            self.fsm = FSM::Connected;
            true
          },
          Err(_) => {
            // NOTE: Setting status and notifying is not necessary- if the send failed there is no app-side connection to observe this or block on it
            self.clear_timers(deps.timers());
            false
          }
        }
      },
      FSM::Connected => {
        // Discard heartbeats (aka empty payloads)
        // TODO: Do we ack if we can't push back?
        if size > header::SIZE_BYTES {
          // TODO: Warn if fails from src buffer too small or dst buffer full?
          buf.push_back(&mut deps.buffer(header::SIZE_BYTES..size)).map(|_| {
            // TODO: Update acks (and call on_packet_acked when not heartbeat)
            buf.notify_one();
          });
          ack((self.local_addr, peer_addr), deps);
        }

        true
      }
    }
  }
}

pub fn ack<D: Deps>(addr_pair: (SocketAddr, SocketAddr), deps: &mut D) {
  let bytes = deps.buffer(header::REMOTE_SEQ_NO_RANGE);
  let ack = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
  deps.on_packet_acked(addr_pair, ack);
}
