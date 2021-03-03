use std::sync::Arc;
use std::net::{UdpSocket, SocketAddr};
use std::io;

use crossbeam::channel::Sender;
use crate::state;

#[allow(non_camel_case_types)]
pub type READ_BUFFER_TAG = ();

// Connection callback on write
pub type OnWrite = dyn Fn(usize) -> io::Result<usize> + Send + Sync;

// Listener callback on close
pub type OnClose = dyn Fn() -> io::Result<()> + Send + Sync;

#[derive(Debug)]
pub enum ToDaemon {
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>, SocketAddr)
}

pub enum FromDaemon {
  Listener(Box<OnClose>),
  Connection(Arc<OnWrite>, Arc<state::Shared>, (SocketAddr, SocketAddr))
}
