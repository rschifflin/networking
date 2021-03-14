use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use bring::bounded::Bring;
use cond_mutex::CondMutexGuard;

use crate::types::FromDaemon as ToService;
use crate::error;
use crate::state::{sequence, util, State, FSM, Sequence, Deps};
use crate::timer::{Timers, TimerKind};
use crate::constants::{header, time_ms};

impl State {
  // Returns false when the connection is terminal and can be cleaned up
  // Returns true otherwise
  pub fn read<D: Deps>(&mut self, local_addr: SocketAddr, peer_addr: SocketAddr, size: usize, deps: &mut D) -> bool {
    let addr_pair = (local_addr, peer_addr);
    let (ref buf_read, ref buf_write, ref status) = *self.shared;

    // TODO: Should we handle a poisoned lock state here? IE if a thread with a connection panics,
    // what should the daemon do about it? Just close the connection?
    // Likely the client should panic on poison, and the daemon should recover the lock and close the conn on poison
    // For now just panic
    let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

    match &mut self.fsm {
      /* Initial read from peer */
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
            /* Initial response handling */
            // This was relevant socket activity, so bump the timeout
            util::bump_timeout(self.socket_id, &mut self.last_recv, deps.now(), deps.timers());
            let mut bytes: [u8; 4] = [0,0,0,0];

            // This was the peer's very first relevant socket activity
            // Peer's very first 'local' sequence # always sets our remote_seq_no
            bytes.copy_from_slice(deps.buffer(header::LOCAL_SEQ_NO_RANGE));
            let seq_no = u32::from_be_bytes(bytes);
            self.sequence.remote_seq_no = seq_no;

            if size > header::SIZE_BYTES {
              // TODO: Warn if fails from src buffer too small or dst buffer full?
              if let Some(_) = buf.push_back(&mut deps.buffer(header::SIZE_BYTES..size)) {
                buf.notify_one();
                drop(buf);
              };
            }

            handle_acks(&mut bytes, &mut self.sequence, addr_pair, deps);
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

      /* Subsequent read from peer */
      FSM::Connected => {
        // Initial read set the remote sequence no.
        // Subsequent reads filter out sequence nos that aren't considered newer
        let mut bytes: [u8; 4] = [0,0,0,0];
        bytes.copy_from_slice(deps.buffer(header::LOCAL_SEQ_NO_RANGE));
        let seq_no = u32::from_be_bytes(bytes);
        let seq_gap = match sequence::distance(self.sequence.remote_seq_no, seq_no) {
          None => return true, // Drop this filtered-out stale packet
          Some(gap) => gap
        };

        util::bump_timeout(self.socket_id, &mut self.last_recv, deps.now(), deps.timers());

        // The connection only sets app_has_hup on drop, which can only occur
        // when all clones have been dropped (they are simply behind an arc).
        // Thus, we can guarantee there are no condvar-listeners to notify
        if status.app_has_hup() {
          // We check the special case of a dropped connection.
          // We can actually clean up the resource if dropped and there are no writes to flush
          let buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
          if buf_write.count() > 0 {
            return true
          } else {
            self.clear_timers(deps.timers());
            return false
          }
        }

        if size <= header::SIZE_BYTES {
          self.sequence.update_remote(seq_no, seq_gap);
        } else {
          // We only ack when we can place the payload in full on the buffer
          // TODO: Warn if fails from src buffer too small or dst buffer full?
          if let Some(_) = buf.push_back(&mut deps.buffer(header::SIZE_BYTES..size)) {
            self.sequence.update_remote(seq_no, seq_gap);
            buf.notify_one();
          };
        }

        handle_acks(&mut bytes, &mut self.sequence, addr_pair, deps);
        true
      }
    }
  }
}

fn handle_acks<D: Deps>(bytes: &mut [u8; 4], sequence: &mut Sequence, addr_pair: (SocketAddr, SocketAddr), deps: &mut D) {
  bytes.copy_from_slice(deps.buffer(header::REMOTE_SEQ_NO_RANGE));
  let ack_no = u32::from_be_bytes(*bytes);
  bytes.copy_from_slice(deps.buffer(header::REMOTE_SEQ_TAIL_RANGE));
  let ack_tail = u32::from_be_bytes(*bytes);
  for ack in sequence.iter_acks(ack_no, ack_tail) {
    deps.on_packet_acked(addr_pair, ack);
  }
}
