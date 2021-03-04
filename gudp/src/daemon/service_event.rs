use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::io;

use mio::Token;

use clock::Clock;

use crate::socket::{Socket, PeerType, ConnOpts};
use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::state::State;
use crate::daemon::{self, poll};
use crate::error;

pub fn handle<C: Clock>(msg: FromService, token_map: &mut HashMap<Token, Socket>, s: &mut daemon::State<C>) {
  match msg {
    FromService::Connect(io, respond_tx, peer_addr) => {
      match poll::register_io(io, s) {
        Some((token, conn, local_addr)) => {
          let conn_opts = ConnOpts::new(token, respond_tx, s.tx_on_write.clone(), Arc::clone(&s.waker));
          let socket_id = (token, peer_addr);
          let state = State::init(socket_id, conn_opts, s);
          let socket = Socket::new(conn, local_addr, PeerType::Direct(peer_addr, state));
          token_map.insert(token, socket);
        }
        None => drop(respond_tx)
      }
    }

    FromService::Listen(io, respond_tx) => {
      match poll::register_io(io, s) {
        Some((token, mut conn, local_addr)) => {
          let on_close = {
            let tx_on_close = s.tx_on_close.clone();
            let waker = Arc::clone(&s.waker);
            move || -> io::Result<()> {
              tx_on_close.send(token).map_err(error::cannot_send_to_daemon)?;
              waker.wake().map_err(error::wake_failed)?;
              Ok(())
            }
          };

          respond_tx.send(ToService::Listener(Box::new(on_close)))
            .map_err(|_| poll::deregister_io(&mut conn, s)).ok()
            .map(|_| {
              let tx_on_write = s.tx_on_write.clone();
              let waker = Arc::clone(&s.waker);
              let peers = HashMap::new();
              let listen = Some(ConnOpts::new(token, respond_tx, tx_on_write, waker));
              let pending_writes = HashSet::new();
              token_map.insert(
                token,
                Socket::new(conn, local_addr, PeerType::Passive { peers, listen, pending_writes })
              );
            });
        },
        None => drop(respond_tx)
      }
    }
  }
}
