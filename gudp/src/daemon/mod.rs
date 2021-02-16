use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::io;
use std::thread;

use mio::{Poll, Events, Token, Waker};
use mio::net::UdpSocket as MioUdpSocket;
use crossbeam::channel;

use crate::constants::WAKE_TOKEN;
use crate::types::ToDaemon as FromService;
use crate::state::State;

mod poll;
mod service_event;
mod read_event;
mod write_event;

pub fn spawn(mut poll: Poll, waker: Arc<Waker>, rx: channel::Receiver<FromService>) -> io::Result<thread::JoinHandle<io::Error>> {
  thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || -> io::Error {
      let (tx_on_write, rx_write_events) = channel::unbounded();
      let mut events = Events::with_capacity(2); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut token_map: HashMap<Token, (MioUdpSocket, SocketAddr)> = HashMap::new();
      let mut states: HashMap<SocketAddr, State> = HashMap::new();

      let timer = std::time::Duration::from_millis(100);
      loop {
        match poll.poll(&mut events, Some(timer)) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(msg, &poll, &mut token_map, &mut states, &tx_on_write, &waker, &mut next_conn_id);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  let (_, addr) = token_entry.get();
                  if let Entry::Occupied(state_entry) = states.entry(*addr) {
                    read_event::handle(token_entry, state_entry, &mut poll);
                  }
                }
              }
            };

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  let (_, addr) = token_entry.get();
                  if let Entry::Occupied(state_entry) = states.entry(*addr) {
                    write_event::handle(token_entry, state_entry, &mut poll);
                  }
                }
              }
            };

            // Handle user writes
            for token in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                let (_, addr) = token_entry.get();
                if let Entry::Occupied(state_entry) = states.entry(*addr) {
                  write_event::handle(token_entry, state_entry, &mut poll);
                }
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut states)
        }
      }
    })
}

