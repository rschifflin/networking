use std::sync::Arc;
use std::net::{UdpSocket, SocketAddr};
use std::io;

use crossbeam::channel::Sender;
use crate::state::{self, Status};
use crate::timer;

#[allow(non_camel_case_types)]
pub type READ_BUFFER_TAG = ();

// Connection callback on write
pub type OnWrite = dyn Fn(usize) -> io::Result<usize> + Send;

// Listener callback on close
pub type OnClose = dyn FnMut() -> io::Result<()> + Send;

// The FSM uses values of this type transparently to register timer events
// Corresponds to the local socket resource plus connected peer address
pub type Expired<'a, T> = timer::Expired<'a, <T as timer::Timers<'a>>::Item>;

//TODO: Listener callback on close
// pub type OnClose = dyn FnMut() + Send;

#[derive(Debug)]
pub enum ToDaemon {
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>, SocketAddr)
}

pub enum FromDaemon {
  Listener(Box<OnClose>),
  Connection(Box<OnWrite>, Arc<state::Shared>, (SocketAddr, SocketAddr))
}
