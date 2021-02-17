use std::sync::{Arc, Mutex};
use std::net::{UdpSocket, SocketAddr};
use std::io;

use crossbeam::channel::Sender;

use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::state::Status;

#[allow(non_camel_case_types)]
pub type READ_BUFFER_TAG = ();

// Conncetion callback on write
pub type OnWrite = dyn Fn(usize) -> io::Result<usize> + Send;

//TODO: Listener callback on close
// pub type OnClose = dyn FnMut() + Send;

pub type SharedConnState = (
  /*BufRead*/   CondMutex<Bring, READ_BUFFER_TAG>,
  /*BufWrite*/  Mutex<Bring>,

  // Atomics
  /*Status*/      Status,
);

#[derive(Debug)]
pub enum ToDaemon {
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>, SocketAddr)
}

pub enum FromDaemon {
  Listener/*TODO: (Box<OnClose>)*/,
  Connection(Box<OnWrite>, Arc<SharedConnState>, (SocketAddr, SocketAddr))
}
