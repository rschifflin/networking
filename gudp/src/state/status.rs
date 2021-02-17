/// Status is a shared state atomic usize with semantic bitflags to indicate
/// statuses about a connection.
/// Importantly, it respects the following invariants:
/// Connections may be either Open or Closed.
/// Writes that change the connection from Open->Open, Open->Closed or Closed->Closed are always allowed.
/// Writes that change the connection from Closed->Open are disallowed.
/// Therefore two clients may race to close a connection, but once a Closed connection is observed, no future writes will ever bring it back to Open.
///
/// Similarly, the OS Error field is for fatal errors and will be set once and only once

use std::sync::atomic::{AtomicI32, AtomicU32};
use std::sync::atomic::Ordering::SeqCst as OSeqCst;
use std::io;

use crate::error;

// The client gracefully dropped their end of the connection
// IO can still be flushed to the socket before the connection ends.
const FLAG_APP_HUP: u32 = 1u32.rotate_right(1);

// The socket gracefully dropped their end of the connection
// This is decided as a consequence of the protocol state
// determining that the virtual connection has timed out.
// IO can still be flushed to the client before the connection ends.
const FLAG_IO_HUP: u32 = 1u32.rotate_right(2);

// The socket encountered an unknown error.
// The raw error code will be available.
// IO can still be flushed to the client before the connection ends.
const FLAG_IO_ERR: u32 = 1u32.rotate_right(3);

// The socket is in the closed state for any reason
const FLAGS_CLOSED: u32 =
  FLAG_APP_HUP |
  FLAG_IO_HUP |
  FLAG_IO_ERR;

// The socket was gracefully closed by either side
const FLAGS_HUP: u32 =
  FLAG_APP_HUP |
  FLAG_IO_HUP;

const FLAGS_APP_CLOSER: u32 = FLAG_APP_HUP;
const FLAGS_PEER_CLOSER: u32 = FLAG_IO_HUP;
const FLAGS_IO_CLOSER: u32 = FLAG_IO_ERR;

const ERRNO_CLEAR: i32 = 0;

#[derive(Debug)]
pub enum ClientStatus { Active }

#[derive(Copy, Clone, Debug)]
pub enum Closer {
  Application,
  Peer,
  IO
}

#[derive(Debug)]
pub struct Status {
  status: AtomicU32,
  errno: AtomicI32,
}

impl Status {
  pub fn new() -> Status {
    Status {
      status: AtomicU32::new(0),
      errno: AtomicI32::new(ERRNO_CLEAR)
    }
  }

  // Indicate the app has gracefully closed their connection end
  pub fn set_client_hup(&self) {
    // Set the app hangup flag and preserve the rest
    self.status.fetch_or(FLAG_APP_HUP, OSeqCst);
  }

  // Indicate the socket has gracefully closed their connection end
  pub fn set_io_hup(&self) {
    // Set the io hangup flag and preserve the rest
    self.status.fetch_or(FLAG_IO_HUP, OSeqCst);
  }

  // Indicate the socket encountered a fatal error.
  // NOTE: The sequencing here is important
  pub fn set_io_err(&self, err: Option<i32>) {
    // Set errno code if unset
    err.map(|errno| {
      self.errno
        .compare_exchange(ERRNO_CLEAR, errno, OSeqCst, OSeqCst)
        .unwrap(); // If the exchange fails for some bizarre reason, we default to an unknown error anyway
    });

    // Set the io err flag and preserve the rest
    self.status.fetch_or(FLAG_IO_ERR, OSeqCst);
  }

  // Will check various flags to determine if the connection qualifes as closed
  // NOTE: Once connections are considered closed, they will never unclose.
  // So if is_closed() == true, you know no races will occur to unclose the conn.
  pub fn is_closed(&self) -> bool {
    (self.status.load(OSeqCst) & FLAGS_CLOSED) != 0
  }

  pub fn test_closed(&self) -> Option<Closer> {
    let status = self.status.load(OSeqCst);
    if (status & FLAGS_APP_CLOSER) != 0 {
      Some(Closer::Application)
    } else if (status & FLAGS_PEER_CLOSER) != 0 {
      Some(Closer::Peer)
    } else if (status & FLAGS_IO_CLOSER) != 0 {
      Some(Closer::IO)
    } else {
      None
    }
  }


  pub fn is_open(&self) -> bool {
    !self.is_closed()
  }

  // TODO: The ClientStatus enum will one day expose semantic status info
  // such as congestion control level, # of dropped packets, etc
  // For now, the only info is that the connection is active.
  pub fn check_client(&self) -> io::Result<ClientStatus> {
    let status = self.status.load(OSeqCst);
    self.check_client_flag_io_err(status)?;
    self.check_client_flags_hup(status)?;
    Ok(ClientStatus::Active)
  }

  fn check_client_flag_io_err(&self, status: u32) -> io::Result<()> {
    if (status & FLAG_IO_ERR) != 0 {
      match self.errno.load(OSeqCst) {
        ERRNO_CLEAR => Err(error::unknown()),
        e => Err(io::Error::from_raw_os_error(e))
      }
    } else {
      Ok(())
    }
  }

  fn check_client_flags_hup(&self, status: u32) -> io::Result<()> {
    if (status & FLAGS_HUP) != 0 {
      Err(error::use_after_hup())
    } else {
      Ok(())
    }
  }
}
