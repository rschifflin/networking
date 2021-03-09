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
use crate::constants::{header, time_ms};

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
    // NOTE: Currently ONLY a timeout can cause a peer_hup, and socket cleanup happens immediately.
    // We will never end up here in the single-threaded event loop writing to a peer which has hung up.
    // So we don't check for peer_hup here. If we add a protocol-level fin message, this may change.

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
      // NOTE: buf_local MUST be large enough to hold the packet header
      s.buf_local[header::MAGIC_BYTES_RANGE].copy_from_slice(&header::MAGIC_BYTES);
      s.buf_local[header::LOCAL_SEQ_NO_RANGE].copy_from_slice(&[0,0,0,0]);
      s.buf_local[header::REMOTE_SEQ_NO_RANGE].copy_from_slice(&[0,0,0,0]);
      s.buf_local[header::REMOTE_SEQ_TAIL_RANGE].copy_from_slice(&[0,0,0,0]);

      // This attempts to peek+send the front blob of the write buffer
      match buf.front(&mut s.buf_local[header::SIZE_BYTES..]).map(|mut front| {
        // TODO: Is it better to provide a &mut WithOpt arg to modify, or to return a tuple of (R, WithOpt) like now?
        front.with(|payload_size_bytes| {
          let send = io.send_to(&s.buf_local[..header::SIZE_BYTES + payload_size_bytes], peer_addr);
          let opt = match send { Ok(_) => WithOpt::Pop, Err(_) => WithOpt::Peek };
          (send, opt)
        })
      }) {
        /* Write OK */
        Some(Ok(total_size_bytes)) => {
          if total_size_bytes > header::SIZE_BYTES {
            if let Some(f) = &mut s.conf.on_packet_sent {
              let bytes = &s.buf_local[header::LOCAL_SEQ_NO_RANGE];
              let sent_seq_no = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
              f((self.local_addr, peer_addr), &s.buf_local[header::SIZE_BYTES..total_size_bytes], sent_seq_no);
            }
          }

          s.timers.remove((self.socket_id, TimerKind::Heartbeat), self.last_send + time_ms::HEARTBEAT);
          let when = s.clock.now();
          self.last_send = when;
          s.timers.add((self.socket_id, TimerKind::Heartbeat), when + time_ms::HEARTBEAT);
        }

        /* Could not peek at the front of the write buffer */
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
