use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use mio::net::UdpSocket as MioUdpSocket;
use bring::bounded::Bring;
use bring::WithOpt;

use crate::error;
use crate::types::READ_BUFFER_TAG;
use crate::types::FromDaemon as ToService;
use crate::state::{State, Status, Closer, FSM};

impl State {
  pub fn write(&mut self, io: &mut MioUdpSocket, peer_addr: SocketAddr, buf_local: &mut [u8]) -> Result<(), Closer> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;
    // TODO: Do we care about status here?

    // TODO: Read in loop until we hit WOULDBLOCK
    let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");

    let buf = &mut *buf_write;
    let send_result = buf.with_front(buf_local, |buf_local, bytes| {
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
      // TODO: If our buf is too small, we should truncate or return Err:WriteZero.
      // Otherwise maybe change buflocal to a vec and only grow it if we get massive packets
      None => Ok(()), // Nothing was on the ring or our buf was too small. Simply no-op the write
      Some(Ok(_)) => Ok(()), // There was data on the buffer and we were able to pop it and send it!
      Some(Err(e)) => {
        // TODO: Add a peer writer map for when we must signal as a writer
        if e.kind() == std::io::ErrorKind::WouldBlock { Ok(()) } // There was data on the buffer but we would've blocked if we tried to send it, so we left it alone
        else {
          // TODO: Handle errors explicitly. Set io_err_x flags based on errorkind
          // Add error flags we can set when we have a semantic error that has no underlying errno code.
          let errno = e.raw_os_error();

          // NOTE: Needed to sync blocked readers before signalling that the connection is closed
          let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");
          status.set_io_err(errno);
          lock.notify_all();
          drop(lock);

          Err(Closer::IO)
        }
      }
    }
  }
}
