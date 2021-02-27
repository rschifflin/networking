use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;
use std::thread;
use std::time::Instant;

use mio::{Poll, Events, Token, Waker};
use crossbeam::channel;

use crate::socket::Socket;
use crate::constants::{WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;
use crate::timer::{self, Timers};

mod poll;
mod service_event;
mod timeout_event;
mod read_event;
mod write_event;
mod listen_close_event;

pub fn spawn(mut poll: Poll, waker: Arc<Waker>, rx: channel::Receiver<FromService>) -> io::Result<thread::JoinHandle<io::Error>> {
  thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || -> io::Error {
      // tx_on_write forwards callbacks from app connections after they call write
      let (tx_on_write, rx_write_events) = channel::unbounded();

      // tx_on_close forwards callbacks from app listeners after they close
      let (tx_on_close, rx_close_listener_events) = channel::unbounded();

      let mut events = Events::with_capacity(128); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut token_map: HashMap<Token, Socket> = HashMap::new();
      let mut buf_local = vec![0u8; CONFIG_BUF_SIZE_BYTES];

      // A hacky alloc to iterate destructively on the keys of the pending_write hashset
      let mut pending_write_keybuf = Vec::with_capacity(128);
      let mut timers: timer::List<(Token, SocketAddr)> = timer::List::new();

      loop {
        let now = Instant::now();
        let timeout = timers.when_next().and_then(|t| t.checked_duration_since(now));
        match poll.poll(&mut events, timeout) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(
                msg,
                &poll,
                &mut token_map,
                &tx_on_write,
                &tx_on_close,
                &waker,
                &mut next_conn_id);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  read_event::handle(token_entry, &mut buf_local, &mut poll, &mut timers);
                }
              }
            };

            // Handle listener close
            for token in rx_close_listener_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                listen_close_event::handle(token_entry, &poll);
              }
            }

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  write_event::handle(token_entry, &mut pending_write_keybuf, &mut buf_local, &mut poll);
                }
              }
            };

            // Handle app writes
            for (token, peer_addr) in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                write_event::handle_app(token_entry, peer_addr, &mut buf_local, &mut poll);
              }
            }

            // Handle timeouts
            // NOTE: Occurs last to allow time to fill the read buffer, last chance to ack heartbeat, etc if necessary
            for (token, peer_addr) in timers.expire(Instant::now()) {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                println!("Got timeout: {:?}", token);
                timeout_event::handle(token_entry, peer_addr, &mut poll);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut token_map)
        }
      }
    })
}

