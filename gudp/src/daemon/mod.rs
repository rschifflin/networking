use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::io;
use std::thread;
use std::time::Duration;

use mio::{Poll, Events, Token, Waker};
use crossbeam::channel;

use clock::Clock;

use crate::socket::{self, Socket};
use crate::constants::{time_ms, WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;
use crate::timer::{self, Timers, TimerKind};
use crate::service::Conf;

pub use state::State;

mod poll;
mod state;
mod service_event;
mod timer_event;
mod read_event;
mod write_event;
mod listen_close_event;

pub fn spawn<C>(
  poll: Poll,
  waker: Arc<Waker>,
  rx: channel::Receiver<FromService>,
  conf: Conf,
  clock: C) -> io::Result<thread::JoinHandle<io::Error>>
where C: 'static + Clock + Send {

  thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || -> io::Error {
      let mut events = Events::with_capacity(1024); // 1024 connections ought to be enough for anybody
      let mut token_map: HashMap<Token, Socket> = HashMap::new();

      // tx_on_write forwards callbacks from app connections after they call write
      let (tx_on_write, rx_write_events) = channel::unbounded();

      // tx_on_close forwards callbacks from app listeners after they close
      let (tx_on_close, rx_close_listener_events) = channel::unbounded();

      let buf_local = vec![0u8; CONFIG_BUF_SIZE_BYTES];
      let timers: timer::List<(socket::Id, TimerKind)> = timer::List::new();

      let mut state = State {
        poll,
        waker,
        tx_on_write,
        tx_on_close,
        next_conn_id: 1,
        buf_local,
        timers,
        conf,
        clock
      };

      // A hacky alloc to iterate with mutation on the keys of the pending_write hashset
      let mut pending_write_keybuf = Vec::with_capacity(1024);
      // A hacky alloc to iterate with mutation on expired timers
      let mut expired_timers = Vec::with_capacity(1024);

      loop {
        let timeout = state.timers.when_next().map(|t| {
          let now = state.clock.now();
          t.checked_duration_since(now)
            .map(|timeout| Duration::max(timeout, time_ms::IOTA))
            .unwrap_or(time_ms::ZERO)
        });

        match state.poll.poll(&mut events, timeout) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(
                msg,
                &mut token_map,
                &mut state);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  read_event::handle(token_entry, &mut state);
                }
              }
            };

            // Handle listener close
            for token in rx_close_listener_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                listen_close_event::handle(token_entry, &mut state);
              }
            }

            // Handle timer expiry
            // NOTE: Occurs after poll read events to allow time to fill the read buffer, last chance to ack heartbeat, etc if necessary
            let now = state.clock.now();
            expired_timers.extend(state.timers.expire(now));
            for ((token, peer_addr), kind) in expired_timers.drain(..) {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                timer_event::handle(token_entry, peer_addr, kind, &mut state);
              }
            }

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  write_event::handle(token_entry, &mut pending_write_keybuf, &mut state);
                }
              }
            };

            // Handle app writes
            for (token, peer_addr) in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                write_event::handle_app(token_entry, peer_addr, &mut state);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut token_map)
        }
      }
    })
}
