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

pub fn no_space_to_read() -> io::Error {
  io::Error::new(io::ErrorKind::UnexpectedEof, "Not enough space to read entire packet")
}

pub fn use_after_hup() -> io::Error {
  io::Error::new(io::ErrorKind::ConnectionReset, "Attempted to use after receiver (or sender) hung up.")
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

pub fn unexpected_recv_from_daemon() -> io::Error {
  io::Error::new(io::ErrorKind::BrokenPipe, "Daemon communicated unexpected message while attempting to recv")
}

pub fn cannot_register_with_daemon() -> io::Error {
  io::Error::new(io::ErrorKind::BrokenPipe, "Daemon failed to acknowledge io registration")
}

pub fn socket_addr_failed_to_resolve() -> io::Error {
  io::Error::new(io::ErrorKind::AddrNotAvailable, "Socket address failed to resolve.")
}
