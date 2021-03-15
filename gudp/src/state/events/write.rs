use std::net::SocketAddr;
use std::io;

use mio::net::UdpSocket as MioUdpSocket;
use bring::WithOpt;
use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::state::{State, Deps, SentSeqNo, util};
use crate::types::READ_BUFFER_TAG;
use crate::constants::{header, SENT_SEQ_BUF_SIZE};

fn terminal<D: Deps>(state: &State, buf_read: &CondMutex<Bring, READ_BUFFER_TAG>, deps: &mut D) -> io::Result<bool> {
  let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
  lock.notify_all();
  state.clear_timers(deps.timers());
  Ok(false)
}

impl State {
  // Returns...
  //    Ok(True) when the state update + write succeeds
  //    Ok(False) when the state has become terminal and the socket can be cleaned up
  //    Err(e) when an io error occurs on write. NOTE: It may be WouldBlock, which is non-fatal

  pub fn write<D: Deps>(&mut self, io: &mut MioUdpSocket, peer_addr: SocketAddr, deps: &mut D) -> io::Result<bool> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    // NOTE: Currently ONLY a timeout can cause a peer_hup, and socket cleanup happens immediately.
    // We will never end up here in the single-threaded event loop writing to a peer which has hung up.
    // So we don't check for peer_hup here. If we add a protocol-level fin message, this may change.

    // loop until we hit WOULDBLOCK, some other err or run out of things to write
    let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");
    loop {
      if buf_write.count() <= 0 {
        // Called with buf_write locked, to prevent a "write then hangup" race
        if status.app_has_hup() { return terminal(self, buf_read, deps); }
        return Ok(true);
      }

      let buf = &mut *buf_write;

      // TODO: Add CRC?
      // NOTE: buf_local MUST be large enough to hold the packet header
      deps.buffer_mut(header::MAGIC_BYTES_RANGE).copy_from_slice(&header::MAGIC_BYTES);
      deps.buffer_mut(header::LOCAL_SEQ_NO_RANGE).copy_from_slice(&self.sequence.local_seq_no.to_be_bytes());
      deps.buffer_mut(header::REMOTE_SEQ_NO_RANGE).copy_from_slice(&self.sequence.remote_seq_no.to_be_bytes());
      deps.buffer_mut(header::REMOTE_SEQ_TAIL_RANGE).copy_from_slice(&self.sequence.remote_seq_tail.to_be_bytes());

      // This attempts to peek+send the front blob of the write buffer
      match buf.front(deps.buffer_mut(header::SIZE_BYTES..)).map(|mut front| {
        front.with(|payload_size_bytes| {
          let send = io.send_to(deps.buffer(..header::SIZE_BYTES + payload_size_bytes), peer_addr);
          let opt = match send { Ok(_) => WithOpt::Pop, Err(_) => WithOpt::Peek };
          (send, opt)
        })
      }) {
        /* Write OK */
        Some(Ok(total_size_bytes)) => {
          let when = deps.now();
          let sent_seq_no = self.sequence.local_seq_no;
          let sent_idx = sent_seq_no as usize % SENT_SEQ_BUF_SIZE;

          // Only notify for contentful packets
          if total_size_bytes > header::SIZE_BYTES {
            deps.on_packet_sent((self.local_addr, peer_addr), header::SIZE_BYTES..total_size_bytes, sent_seq_no);
            self.sequence.sent_seq_buf[sent_idx] = Some(SentSeqNo::new(sent_seq_no, when));
          } else {
            // Erase the 'old' buffer entry for rare cases of high packet loss leading to unintentional acks
            self.sequence.sent_seq_buf[sent_idx] = None;
          }

          util::bump_heartbeat(self.socket_id, &mut self.last_send, when, deps.timers());

          // Bump to the next unsent sequence number
          self.sequence.local_seq_no = self.sequence.local_seq_no.wrapping_add(1);
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
