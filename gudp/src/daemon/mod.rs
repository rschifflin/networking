use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use mio::{Poll, Events, Token, Waker};
use crossbeam::channel;

use crate::socket::{self, Socket};
use crate::constants::{time_ms, WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;
use crate::timer::{self, Timers, TimerKind, SystemClock};

mod poll;
mod service_event;
mod timer_event;
mod read_event;
mod write_event;
mod listen_close_event;

// Contains all the state used by the single threaded event loop handlers and state changes
pub struct LoopLocalState {
  pub poll: Poll,
  pub waker: Arc<Waker>,
  pub tx_on_write: channel::Sender<socket::Id>,
  pub tx_on_close: channel::Sender<Token>,
  pub next_conn_id: usize,
  pub buf_local: Vec<u8>,
  pub timers: timer::List<(socket::Id, TimerKind)>,
  pub clock: SystemClock
}

pub fn spawn(poll: Poll, waker: Arc<Waker>, rx: channel::Receiver<FromService>) -> io::Result<thread::JoinHandle<io::Error>> {
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

      let mut loop_local_state = LoopLocalState {
        poll,
        waker,
        tx_on_write,
        tx_on_close,
        next_conn_id: 1,
        buf_local,
        timers,
        clock: SystemClock()
      };

      // A hacky alloc to iterate with mutation on the keys of the pending_write hashset
      let mut pending_write_keybuf = Vec::with_capacity(1024);
      // A hacky alloc to iterate with mutation on expired timers
      let mut expired_timers = Vec::with_capacity(1024);

      loop {
        let timeout = loop_local_state.timers.when_next().map(|t| {
          let now = Instant::now();
          t.checked_duration_since(now)
            .map(|timeout| Duration::max(timeout, time_ms::IOTA))
            .unwrap_or(time_ms::ZERO)
        });

        match loop_local_state.poll.poll(&mut events, timeout) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(
                msg,
                &mut token_map,
                &mut loop_local_state);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  read_event::handle(token_entry, &mut loop_local_state);
                }
              }
            };

            // Handle listener close
            for token in rx_close_listener_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                listen_close_event::handle(token_entry, &mut loop_local_state);
              }
            }

            // Handle timer expiry
            // NOTE: Occurs after poll read events to allow time to fill the read buffer, last chance to ack heartbeat, etc if necessary
            expired_timers.extend(loop_local_state.timers.expire(Instant::now()));
            for ((token, peer_addr), kind) in expired_timers.drain(..) {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                timer_event::handle(token_entry, peer_addr, kind, &mut loop_local_state);
              }
            }

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  write_event::handle(token_entry, &mut pending_write_keybuf, &mut loop_local_state);
                }
              }
            };

            // Handle app writes
            for (token, peer_addr) in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                write_event::handle_app(token_entry, peer_addr, &mut loop_local_state);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut token_map)
        }
      }
    })
}
