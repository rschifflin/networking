use std::slice::SliceIndex;
use std::net::SocketAddr;
use std::sync::Arc;

use crossbeam::channel;
use mio::{Poll, Token, Waker};
use clock::Clock;

use crate::service::Conf;
use crate::socket;
use crate::timer::{self, TimerKind};
use crate::state::Deps;
use crate::warn;

// Contains all the state used by the single threaded event loop handlers and state changes
pub struct State<C: Clock> {
  pub poll: Poll,
  pub waker: Arc<Waker>,
  pub tx_on_write: channel::Sender<socket::Id>,
  pub tx_on_close: channel::Sender<Token>,
  pub next_conn_id: usize,
  pub buf_local: Vec<u8>,
  pub timers: timer::List<(socket::Id, TimerKind)>,
  pub conf: Conf,
  pub clock: C
}

impl<C: Clock> Deps for State<C> {
  fn timers(&mut self) -> &mut timer::List<(socket::Id, TimerKind)> {
    &mut self.timers
  }

  fn now(&mut self) -> std::time::Instant {
    self.clock.now()
  }

  fn buffer<I: SliceIndex<[u8], Output = [u8]>>(&self, index: I) -> &[u8] {
    &self.buf_local[index]
  }

  fn buffer_mut<I: SliceIndex<[u8], Output = [u8]>>(&mut self, index: I) -> &mut [u8] {
    &mut self.buf_local[index]
  }

  fn notify_write(&self, socket_id: socket::Id) {
    self.tx_on_write.send(socket_id).unwrap_or_else(warn::tx_to_write_send_failed);
  }

  fn on_packet_sent<I>(&mut self, addr_pair: (SocketAddr, SocketAddr), index: I, sequence_no: u32)
  where I: SliceIndex<[u8], Output = [u8]> {
    let buf = &self.buf_local[index];
    self.conf.on_packet_sent.as_mut().map(|f| f(addr_pair, buf, sequence_no));
  }

  fn on_packet_acked(&mut self, addr_pair: (SocketAddr, SocketAddr), sequence_no: u32) {
    self.conf.on_packet_acked.as_mut().map(|f| f(addr_pair, sequence_no));
  }

  fn conf(&self) -> &Conf {
    &self.conf
  }
}
