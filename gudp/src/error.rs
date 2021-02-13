use std::sync::PoisonError;
use std::io;
use crossbeam::channel::{RecvError, SendError};

pub fn poisoned_write_lock<_T>(_: PoisonError<_T>) -> io::Error {
  io::Error::new(io::ErrorKind::Other, "Write buffer lock was poisoned. Can not continue.")
}

pub fn poisoned_read_lock<_T>(_: PoisonError<_T>) -> io::Error {
  io::Error::new(io::ErrorKind::Other, "Read buffer lock was poisoned. Can not continue.")
}

pub fn wake_failed(reason: io::Error) -> io::Error {
  io::Error::new(io::ErrorKind::Other, reason)
}

pub fn no_space_to_write() -> io::Error {
  io::Error::new(io::ErrorKind::WriteZero, "Not enough space to write entire packet")
}

pub fn no_space_to_read() -> io::Error {
  io::Error::new(io::ErrorKind::UnexpectedEof, "Not enough space to read entire packet")
}

pub fn use_after_hup() -> io::Error {
  io::Error::new(io::ErrorKind::ConnectionReset, "Attempted to use after graceful close.")
}

pub fn unknown() -> io::Error {
  io::Error::new(io::ErrorKind::Other, "An unknown IO error occured")
}

pub fn cannot_send_to_daemon<T: 'static + Sync + Send>(reason: SendError<T>) -> io::Error {
  io::Error::new(io::ErrorKind::BrokenPipe, reason)
}

pub fn cannot_recv_from_daemon(reason: RecvError) -> io::Error {
  io::Error::new(io::ErrorKind::BrokenPipe, reason)
}
