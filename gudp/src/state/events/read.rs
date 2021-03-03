use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use crate::types::FromDaemon as ToService;
use crate::error;
use crate::state::{State, FSM};
use crate::timer::{Timers, TimerKind, Clock};
use crate::daemon::LoopLocalState;
use crate::constants::time_ms;

impl State {
  // Returns true when the connection is updated
  // Returns false when the connection is terminal and can be cleaned up
  pub fn read(&mut self, local_addr: SocketAddr, peer_addr: SocketAddr, size: usize, s: &mut LoopLocalState) -> bool {
    s.timers.remove((self.socket_id, TimerKind::Timeout), self.last_recv + time_ms::TIMEOUT);
    let when = s.clock.now();
    self.last_recv = when;
    s.timers.add((self.socket_id, TimerKind::Timeout), when + time_ms::TIMEOUT);

    let (ref buf_read, ref buf_write, ref status) = *self.shared;

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
    if status.app_has_hup() {
      buf.notify_all();
      let buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
      if buf_write.count() > 0 {
        return true
      } else {
        self.clear_timers(s);
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
            // TODO: Warn if fails from src buffer too small or dst buffer full?
            buf.push_back(&mut s.buf_local[..size]).map(|_| buf.notify_one());
            self.fsm = FSM::Connected;
            true
          },
          Err(_) => {
            // NOTE: Setting status and notifying is not necessary- if the send failed there is no app-side connection to observe this or block on it
            self.clear_timers(s);
            false
          }
        }
      },
      FSM::Connected => {
        // TODO: Warn if fails from src buffer too small or dst buffer full?
        buf.push_back(&mut s.buf_local[..size]).map(|_| buf.notify_one());
        true
      }
    }
  }
}
