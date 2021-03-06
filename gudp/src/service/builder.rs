use std::io;
use std::net::SocketAddr;

use clock::Clock;

use super::{Conf, Service};


// NOTE: If we had generic specialization, this would not need 2 separate structs
// NOTE: If we had a delegate pattern, we wouldnt have to have two identical impls
// Instead, we make two separate structs and macro-ize their shared code
pub struct Builder { conf: Conf }
pub struct ClockedBuilder<C: 'static + Clock + Send> { clock: C, conf: Conf }

macro_rules! impl_builder {
  ( $builder:ty ) => {
    pub fn example(mut self, example: usize) -> $builder {
      self.conf.example = example;
      self
    }

    pub fn on_packet_sent(mut self, f: Box<dyn FnMut((SocketAddr, SocketAddr), &[u8], u32) + Send>) -> $builder {
      self.conf.on_packet_sent = Some(f);
      self
    }

    pub fn on_packet_acked(mut self, f: Box<dyn FnMut((SocketAddr, SocketAddr), u32) + Send>) -> $builder {
      self.conf.on_packet_acked = Some(f);
      self
    }

    pub fn on_packet_lost(mut self, f: Box<dyn FnMut((SocketAddr, SocketAddr), u32) + Send>) -> $builder {
      self.conf.on_packet_lost = Some(f);
      self
    }
  }
}

// Default case with system clock
impl Builder {
  impl_builder!(Builder);

  pub fn new() -> Builder {
    Builder { conf: Conf::default() }
  }

  pub fn clock<C: 'static + Clock + Send>(self, clock: C) -> ClockedBuilder<C> {
    ClockedBuilder {
      conf: self.conf,
      clock
    }
  }

  pub fn build(self) -> io::Result<Service> {
    Service::initialize(self.conf)
  }
}

// Custom clock case
impl <C: 'static + Clock + Send> ClockedBuilder<C> {
  impl_builder!(ClockedBuilder<C>);

  pub fn clock<C2: 'static + Clock + Send>(self, clock: C2) -> ClockedBuilder<C2> {
    ClockedBuilder {
      conf: self.conf,
      clock
    }
  }

  pub fn build(self) -> io::Result<Service> {
    Service::initialize_with_clock(self.conf, self.clock)
  }
}

