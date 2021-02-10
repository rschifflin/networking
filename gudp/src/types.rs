use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex, Condvar};
use std::net::UdpSocket;
use bring::Bring;

pub type SharedConnState = (/*BufRead*/ Mutex<Bring>, /*BufWrite*/ Mutex<Bring>, /*ReadCond*/ Condvar);

#[derive(Debug)]
pub enum ToDaemon {
  Print(&'static str),
  Listen(UdpSocket, Sender<FromDaemon>),
  Connect(UdpSocket, Sender<FromDaemon>),
}

#[derive(Debug)]
pub enum FromDaemon {
  Connection(Arc<SharedConnState>)
}

