/// Trait used to represent external dependencies passed in for each state transition.
/// It represents shared tools like a pre-allocated buffer, a Timer list, a clock, and so forth

use std::slice::SliceIndex;
use std::net::SocketAddr;
use std::time::Instant;
// TODO: Simply export Timer instead of TimerList and change the underlying impl in the timer crate, not generically
use crate::timer::{self, TimerKind};
use crate::socket;
use crate::service;

pub trait Deps {
  fn timers(&mut self) -> &mut timer::List<(socket::Id, TimerKind)>;
  fn now(&mut self) -> Instant;

  fn buffer<I>(&self, index: I) -> &[u8]
  where I: SliceIndex<[u8], Output = [u8]>;

  fn buffer_mut<I>(&mut self, index: I) -> &mut [u8]
  where I: SliceIndex<[u8], Output = [u8]>;

  fn on_packet_sent<I>(&mut self, addr_pair: (SocketAddr, SocketAddr), index: I, sequence_no: u32)
  where I: SliceIndex<[u8], Output = [u8]>;

  fn on_packet_acked(&mut self, addr_pair: (SocketAddr, SocketAddr), sequence_no: u32);

  fn notify_write(&self, socket_id: socket::Id);
  fn conf(&self) -> &service::Conf;
}
