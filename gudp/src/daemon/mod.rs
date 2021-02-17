use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::io;
use std::thread;

use mio::{Poll, Events, Token, Waker};
use crossbeam::channel;

use crate::socket::{Socket, PeerType};
use crate::constants::{WAKE_TOKEN, CONFIG_BUF_SIZE_BYTES};
use crate::types::ToDaemon as FromService;

mod poll;
mod service_event;
mod read_event;
mod write_event;

pub fn spawn(mut poll: Poll, waker: Arc<Waker>, rx: channel::Receiver<FromService>) -> io::Result<thread::JoinHandle<io::Error>> {
  thread::Builder::new()
    .name("gudp daemon".to_string())
    .spawn(move || -> io::Error {
      let (tx_on_write, rx_write_events) = channel::unbounded();
      let mut events = Events::with_capacity(128); // 128 connections ought to be enough for anybody
      let mut next_conn_id = 1;
      let mut token_map: HashMap<Token, Socket> = HashMap::new();
      let mut buf_local = vec![0u8; CONFIG_BUF_SIZE_BYTES];

      loop {
        match poll.poll(&mut events, None) {
          Ok(()) => {
            // Clear out all msgs from service
            for msg in rx.try_iter() {
              service_event::handle(msg, &poll, &mut token_map, &tx_on_write, &waker, &mut next_conn_id);
            }

            // Handle reads
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_readable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  read_event::handle(token_entry, &mut buf_local, &mut poll);
                }
              }
            };

            // Handle poll writeable
            for event in events.iter() {
              if event.token() != WAKE_TOKEN && event.is_writable() {
                if let Entry::Occupied(token_entry) = token_map.entry(event.token()) {
                  let socket = token_entry.get();
                  match socket.peer_type {
                    PeerType::Passive { .. /* peers, listen */} => { /* ... */ },
                    PeerType::Direct(ref peer_addr, ref _state) => {
                      let peer_addr = *peer_addr;
                      drop(socket);
                      write_event::handle(token_entry, peer_addr, &mut buf_local, &mut poll);
                    }
                  }
                }
              }
            };

            // Handle user writes
            for (token, peer_addr) in rx_write_events.try_iter() {
              if let Entry::Occupied(token_entry) = token_map.entry(token) {
                write_event::handle(token_entry, peer_addr, &mut buf_local, &mut poll);
              }
            }
          },

          Err(e) => return poll::handle_failure(e, &mut token_map)
        }
      }
    })
}

