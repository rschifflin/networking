use std::net::SocketAddr;
use std::io;

use mio::net::UdpSocket as MioUdpSocket;
use bring::WithOpt;
use bring::bounded::Bring;
use log::trace;

use cond_mutex::CondMutex;
use clock::Clock;

use crate::timer::{Timers, TimerKind};
use crate::state::State;
use crate::types::READ_BUFFER_TAG;
use crate::daemon;
use crate::constants::time_ms;

fn terminal<C: Clock>(state: &State, buf_read: &CondMutex<Bring, READ_BUFFER_TAG>, s: &mut daemon::State<C>) -> io::Result<bool> {
  let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
  lock.notify_all();
  state.clear_timers(s);
  Ok(false)
}

impl State {
  // Returns...
  //    Ok(True) when the state update + write succeeds
  //    Ok(False) when the state has become terminal and the socket can be cleaned up
  //    Err(e) when an io error occurs on write. NOTE: It may be WouldBlock, which is non-fatal

  pub fn write<C: Clock>(&mut self, io: &mut MioUdpSocket, peer_addr: SocketAddr, s: &mut daemon::State<C>) -> io::Result<bool> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    // NOTE: Currently ONLY a timeout can cause a peer_hup, and socket cleanup happens immediately
    // So we don't check for peer_hup here.

    // loop until we hit WOULDBLOCK, some other err or run out of things to write
    let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
    loop {
      if buf_write.count() <= 0 {
        // Called with buf_write locked, to prevent a "write then hangup" race
        if status.app_has_hup() { return terminal(self, buf_read, s); }
        return Ok(true);
      }

      let buf = &mut *buf_write;

      // TODO: Prefix buf_local with header, seq no, etc
      let send_result = buf.with_front(&mut s.buf_local, |buf_local, bytes| {
        let send = io.send_to(&buf_local[..bytes], peer_addr);
        let opt = match send {
          Ok(_) => WithOpt::Pop,
          Err(_) => WithOpt::Peek
        };
        (send, opt)
      });

      match send_result {
        /* Write OK */
        Some(Ok(size)) => {
          if let Some(f) = &mut s.conf.on_packet_sent { f((self.local_addr, peer_addr), &s.buf_local[..size], 0); }

          s.timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + time_ms::HEARTBEAT);
          let when = s.clock.now();
          self.last_send = when;
          s.timers.add((self.socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
        }

        /* Write buffer issue */
        // TODO: If our buf is too small, should we truncate? return Err:WriteZero?
        // Otherwise maybe change buflocal to a vec and only grow it if we get massive packets?
        None => buf_write.clear(), // For now just empty the buffer

        /* Write Err */
        // This may be a safe WouldBlock. Err results do NOT indicate that listeners have been notified/timers cleared, etc.
        // To ensure proper cleanup, it is up to the caller to call `on_io_err` on this state machine if the error is indeed fatal.
        Some(Err(e)) => return Err(e)
      }
    }
  }
}
