use std::net::SocketAddr;
use std::io;

use mio::net::UdpSocket as MioUdpSocket;
use bring::WithOpt;

use crate::state::State;
use crate::timer::{Timers, TimerKind, Clock};
use crate::daemon::LoopLocalState;
use crate::constants::time_ms;

impl State {
  // Returns...
  //    Ok(True) when the state update + write succeeds
  //    Ok(False) when the state has become terminal and the socket can be cleaned up
  //    Err(e) when an io error occurs on write. NOTE: It may be WouldBlock, which is non-fatal

  pub fn write(&mut self, io: &mut MioUdpSocket, peer_addr: SocketAddr, s: &mut LoopLocalState) -> io::Result<bool> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");

    // Lock not needed- only set by the single-threaded event loop, no concurrent races
    if status.peer_has_hup() {
      let buf_read = buf_read.lock().expect("Could not acquire unpoisoned read lock");

      // If peer has hung up and nothing is left to read, we can signal and clean up
      if buf_read.count() == 0 {
        buf_read.notify_all();
        self.clear_timers(s);
        return Ok(false)
      } else {
        return Ok(true)
      }
    }

    // loop until we hit WOULDBLOCK, some other err or run out of things to write
    loop {
      if buf_write.count() <= 0 { return Ok(true); }

      let buf = &mut *buf_write;
      let send_result = buf.with_front(&mut s.buf_local, |buf_local, bytes| {
        let send = io.send_to(&buf_local[..bytes], peer_addr);
        let opt = match send {
          Ok(_) => WithOpt::Pop,
          Err(_) => WithOpt::Peek
        };
        (send, opt)
      });

      match send_result {
        // TODO: If our buf is too small, should we truncate? return Err:WriteZero?
        // Otherwise maybe change buflocal to a vec and only grow it if we get massive packets?
        None => buf_write.clear(), // For now just empty the buffer
        Some(Ok(_)) => { // There was data on the buffer and we were able to pop it and send it!
          s.timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + time_ms::HEARTBEAT);
          let when = s.clock.now();
          self.last_send = when;
          s.timers.add((self.socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
        },
        Some(Err(e)) => {
          return Err(e);
        }
      }
    }
  }
}
