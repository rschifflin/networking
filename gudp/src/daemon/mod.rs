use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use mio::{Poll, Events, Token, Waker};
use mio::net::UdpSocket as MioUdpSocket;
use crossbeam::channel::Receiver;

use crate::constants::WAKE_TOKEN;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::hashmap_ext;

mod service_event;
mod read_event;
mod write_event;

pub fn spawn(mut poll: Poll, _waker: Arc<Waker>, rx: Receiver<FromService>) {
  std::thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || {
      let mut events = Events::with_capacity(2); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut states: HashMap<Token, (State, MioUdpSocket)> = HashMap::new();
      // NOTE: This is a pre-emptive alloc to ease the burden of iterating over a mutable hashmap
      let mut states_keybuf: Vec<Token> = vec![];

      // Clear out all msgs from service
      loop {
        poll.poll(&mut events, None).expect("Could not poll");

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

        // Handle writes
        for key in hashmap_ext::keys_iter(&states, &mut states_keybuf) {
          if let Entry::Occupied(entry) = states.entry(key) {
            write_event::handle(entry, &mut poll);
          }
        }
      }
    })
    .expect("Could not spawn daemon");
}

