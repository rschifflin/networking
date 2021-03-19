use std::sync::atomic::AtomicU32;
use std::time::Duration;

const RTT_SMOOTHING_FACTOR: f32 = 0.25;

pub struct Shared {
  pub rtt: AtomicU32,
  pub loss: AtomicU32
}

pub struct NetStat {
  pub rtt: Rtt,
  pub loss: Loss
}

impl NetStat {
  pub fn new(baseline_rtt: u32) -> NetStat {
    NetStat {
      rtt: Rtt::new(baseline_rtt),
      loss: Loss::new()
    }
  }
}

/// Round-trip time- exponentially weighted moving average
pub struct Rtt {
  prediction: f32
}

impl Rtt {
  pub fn new(baseline_ms: u32) -> Rtt {
    Rtt {
      prediction: baseline_ms as f32
    }
  }

  pub fn measure(&mut self, rtt: Duration) -> u32 {
    self.prediction =
      self.prediction + RTT_SMOOTHING_FACTOR * ((rtt.as_millis() as f32) - self.prediction);

    self.prediction.max(0.0).floor() as u32
  }
}

/// Packet loss % estimate- alltime
pub struct Loss {
  last_n_packets: u32
}

impl Loss {
  pub fn new() -> Loss {
    Loss {
      last_n_packets: 0
    }
  }

  pub fn lost(&mut self, amount: u32) -> u32 { self.update(true, amount) }
  pub fn found(&mut self, amount: u32) -> u32 { self.update(false, amount) }

  fn update(&mut self, was_lost: bool, amount: u32) -> u32 {
    match amount {
      0 => { /* do nothing */ },
      1..=31 => {
        self.last_n_packets <<= amount;
        if was_lost {
          let fill_ones = 2u32.pow(amount) - 1;
          self.last_n_packets |= fill_ones;
        }
      },
      _gte_32 => {
        if was_lost { self.last_n_packets = u32::MAX; } else { self.last_n_packets = 0; }
      }
    }

    (self.last_n_packets.count_ones() * 100) / 32
  }
}
