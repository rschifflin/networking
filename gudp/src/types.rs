use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex};
use std::net::UdpSocket;
use bring::Bring;

pub type SharedRingBuf = Arc<Mutex<Bring>>;

#[derive(Debug)]
pub enum ToDaemon {
  Print(&'static str),
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>),
}

#[derive(Debug)]
pub enum FromDaemon {
  Connection(/*BufRead*/ SharedRingBuf, /*BufWrite*/ SharedRingBuf)
}

