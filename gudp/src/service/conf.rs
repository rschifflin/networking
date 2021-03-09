use std::net::SocketAddr;

#[derive(Default)]
pub struct Conf {
  pub example: usize,

  // Called when the packet is sent over the wire, with its sequence number
  pub on_packet_sent: Option<Box<dyn FnMut((SocketAddr, SocketAddr), &[u8], usize) + Send>>,

  // Called when the given sequence number is acked
  pub on_packet_acked: Option<Box<dyn FnMut((SocketAddr, SocketAddr), usize) + Send>>,

  // Called when the given sequence number is lost (never acked and too old)
  pub on_packet_lost: Option<Box<dyn FnMut((SocketAddr, SocketAddr), usize) + Send>>
}
