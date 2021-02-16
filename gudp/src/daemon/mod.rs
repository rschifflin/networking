use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::io;
use std::thread;

use mio::{Poll, Events, Token, Waker};
use mio::net::UdpSocket as MioUdpSocket;
use crossbeam::channel::Receiver;

use crate::constants::WAKE_TOKEN;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::hashmap_ext::HashMapExt;

mod poll;
mod service_event;
mod read_event;
mod write_event;

pub fn spawn(mut poll: Poll, _waker: Arc<Waker>, rx: Receiver<FromService>) -> io::Result<thread::JoinHandle<io::Error>> {
  thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || -> io::Error {
      let mut events = Events::with_capacity(2); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut states: HashMap<Token, (State, MioUdpSocket)> = HashMap::new();
      // NOTE: This is a pre-emptive alloc to ease the burden of iterating over a mutable hashmap
      let mut states_keybuf: Vec<Token> = vec![];
      let timer = std::time::Duration::from_millis(100);
      loop {
        match poll.poll(&mut events, Some(timer)) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(msg, &poll, &mut states, &mut next_conn_id);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(entry) = states.entry(event.token()) {
                  read_event::handle(entry, &mut poll);
                }
              }
            };

            // TODO: Add signal_write chan and only update signalled writers
            // TODO: Add timer wheel iter and only update on timed out writers
            // Handle writes
            for key in states.keys_ext(&mut states_keybuf) {
              if let Entry::Occupied(entry) = states.entry(key) {
                write_event::handle(entry, &mut poll);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut states)
        }
      }
    })
}

