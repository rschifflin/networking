use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex};
use std::net::UdpSocket;
use bring::Bring;

use cond_mutex::CondMutex;
use crate::state::Status;

#[allow(non_camel_case_types)]
pub type READ_BUFFER_TAG = ();

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

#[derive(Debug)]
pub enum FromDaemon {
  IORegistered,
  Connection(Arc<SharedConnState>)
}
