use std::sync::{Arc, Mutex};
use std::net::UdpSocket;
use std::io;

use crossbeam::channel::Sender;

use bring::bounded::Bring;
use cond_mutex::CondMutex;

use crate::state::Status;

#[allow(non_camel_case_types)]
pub type READ_BUFFER_TAG = ();

//TODO: Match on pairs of local:peer addrs; pub type AddrPair = (/*Local*/ SocketAddr, /*Remote*/ SocketAddr);

pub type OnWrite = dyn Fn(usize) -> io::Result<usize> + Send;
pub type SharedConnState = (
  /*BufRead*/   CondMutex<Bring, READ_BUFFER_TAG>,
  /*BufWrite*/  Mutex<Bring>,

  // Atomics
  /*Status*/      Status,
);

#[derive(Debug)]
pub enum ToDaemon {
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>)
}

//TODO: Put debug bound on F #[derive(Debug)]
pub enum FromDaemon {
  IORegistered,
  Connection(Box<OnWrite>, Arc<SharedConnState>)
}
