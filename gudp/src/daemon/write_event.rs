use std::collections::hash_map::OccupiedEntry;
use std::net::SocketAddr;

use mio::{Poll, Token};

use bring::WithOpt;

use crate::socket::{Socket, PeerType};
use crate::daemon::poll;

type TokenEntry<'a> = OccupiedEntry<'a, Token, Socket>;
pub fn handle(mut token_entry: TokenEntry, peer_addr: SocketAddr, buf_local: &mut [u8], poll: &Poll) {
  let socket = token_entry.get_mut();
  match &mut socket.peer_type {
    PeerType::Passive { peers, listen } => {
      match (peers.get_mut(&peer_addr), listen) {
        (Some(mut state), _) => { /* handle the direct case as below */ },
        (None, _) => { /* discard socket noise */ },
      }
    },

    PeerType::Direct(addr, state) => {
      // TODO: Do we care if addr != peer_addr here?

      let io = &mut socket.io;
      let (ref buf_read, ref buf_write, ref status) = *state.shared;
      // NOTE: Unlike the READ case, WRITEs never sleep on a condvar. If a write would overflow the write buffer, we
      // return an Err::WriteZero immediately instead.
      // One open question is SHOULD we add a block-on-write interface? (Leaning towards yes for completeness)
      // Doing so would add another condvar and require us to acquire BOTH the read+write locks before deregistering,
      // signalling both condvars.

      // TODO: Read in loop until we hit WOULDBLOCK
      let mut buf_write = buf_write.lock().expect("Could not acquire unpoisoned write lock");

      let buf = &mut *buf_write;
      let send_result = buf.with_front(buf_local, |buf_local, bytes| {
        let send = io.send_to(&buf_local[..bytes], *addr);
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
        None => (), // Nothing was on the ring or our buf was too small. Simply no-op the write
        Some(Ok(_)) => (), // There was data on the buffer and we were able to pop it and send it!
        Some(Err(e)) => {
          if e.kind() == std::io::ErrorKind::WouldBlock {} // There was data on the buffer but we would've blocked if we tried to send it, so we left it alone
          else {
            // TODO: Handle errors explicitly. Set io_err_x flags based on errorkind
            // Add error flags we can set when we have a semantic error that has no underlying errno code.
            let errno = e.raw_os_error();

            // NOTE: Needed to sync blocked readers before signalling that the connection is closed
            let lock = buf_read.lock().expect("Could not acquire unpoisoned read lock");

            status.set_io_err(errno);
            lock.notify_all();
            poll::deregister_io(poll, io);
            drop(lock);
            token_entry.remove();
          }
        }
      }
    }
  }
}
