use std::net::SocketAddr;
use std::io;
use std::time::Instant;

use mio::net::UdpSocket as MioUdpSocket;
use bring::WithOpt;

use crate::state::State;
use crate::timer::{Timers, TimerKind};
use crate::daemon::LoopLocalState;

// TODO: Give state a SystemClock instead of passing in whens...
impl State {
  // Returns...
  //    Ok(True) when the state update + write succeeds
  //    Ok(False) when the state update succeeds but write must block
  //    Err(e) when an io error (other than WouldBlock) occurs on write
  pub fn write(&mut self, io: &mut MioUdpSocket, peer_addr: SocketAddr, when: Instant, s: &mut LoopLocalState) -> io::Result<bool> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    // TODO: Handle appropriate flushing behavior on closed ends:
    //    - if peer is closed, discard writes and remove right away (app can still drain read buffer, app writes will fail)
    //    - if app is closed, do not remove until writes are flushed
    //    - if io is closed, remove right away and remove all siblings

    // TODO: Read in loop until we hit WOULDBLOCK
    let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");

    let buf = &mut *buf_write;
    let send_result = buf.with_front(&mut s.buf_local, |buf_local, bytes| {
      let send = io.send_to(&buf_local[..bytes], peer_addr);
      let opt = match send {
        Ok(_) => WithOpt::Pop,
        Err(_) => WithOpt::Peek
      };
      (send, opt)
    });
    drop(buf);
    drop(buf_write);

    match send_result {
      // TODO: If our buf is too small, should we truncate? return Err:WriteZero?
      // Otherwise maybe change buflocal to a vec and only grow it if we get massive packets
      None => Ok(true), // Nothing was on the ring or our buf was too small. Simply no-op the write
      Some(Ok(_)) => {
        s.timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + std::time::Duration::from_millis(1_000));
        self.last_send = when;
        s.timers.add((self.socket_id, TimerKind::Heartbeat), when + std::time::Duration::from_millis(1_000));
        Ok(true)
      }, // There was data on the buffer and we were able to pop it and send it!
      Some(Err(e)) => {
        if e.kind() == std::io::ErrorKind::WouldBlock { Ok(false) } // There was data on the buffer but we would've blocked if we tried to send it, so we left it alone
        else {
          // TODO: Handle errors explicitly. Set io_err_x flags based on errorkind
          // Add error flags we can set when we have a semantic error that has no underlying errno code.
          let errno = e.raw_os_error();

          // NOTE: Need to sync blocked readers by locking before signalling that the connection is closed
          let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
          status.set_io_err(errno);
          lock.notify_all();
          drop(lock);

          Err(e)
        }
      }
    }
  }
}
