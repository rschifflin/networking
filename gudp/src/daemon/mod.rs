use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use mio::{Poll, Events, Token, Waker};
use crossbeam::channel;

use crate::socket::{self, Socket};
use crate::constants::{WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;
use crate::timer::{self, Timers, TimerKind};

const TIME_ZERO: Duration = Duration::from_millis(0);
const TIME_IOTA: Duration = Duration::from_millis(10);

mod poll;
mod service_event;
mod timer_event;
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

      // TODO: Bundle up event-loop-local state behind single mut ref, since each handler will only access it one at a time anyway,
      // especially poll, timers, channels, buf_local, local vecs... since the fn signatures are getting huge
      let mut events = Events::with_capacity(128); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut token_map: HashMap<Token, Socket> = HashMap::new();
      let mut buf_local = vec![0u8; CONFIG_BUF_SIZE_BYTES];
      let mut timers: timer::List<(socket::Id, TimerKind)> = timer::List::new();

      // A hacky alloc to iterate with mutation on the keys of the pending_write hashset
      let mut pending_write_keybuf = Vec::with_capacity(128);
      // A hacky alloc to iterate with mutation on expired timers
      let mut expired_timers = Vec::with_capacity(128);

      loop {
        let timeout = timers.when_next().map(|t| {
          let now = Instant::now();
          t.checked_duration_since(now)
            .map(|timeout| Duration::max(timeout, TIME_IOTA))
            .unwrap_or(TIME_ZERO)
        });

        match poll.poll(&mut events, timeout) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(
                msg,
                &poll,
                &mut timers,
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

            // Handle timer expiry
            // NOTE: Occurs after poll events to allow time to fill the read buffer, last chance to ack heartbeat, etc if necessary
            expired_timers.extend(timers.expire(Instant::now()));
            for ((token, peer_addr), kind) in expired_timers.drain(..) {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                timer_event::handle(token_entry, peer_addr, &mut buf_local, kind, &tx_on_write, &mut poll, &mut timers);
              }
            }

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  write_event::handle(token_entry, &mut pending_write_keybuf, &mut buf_local, &mut poll, &mut timers);
                }
              }
            };

            // Handle app writes
            for (token, peer_addr) in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                write_event::handle_app(token_entry, peer_addr, &mut buf_local, &mut poll, &mut timers);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut token_map)
        }
      }
    })
}
