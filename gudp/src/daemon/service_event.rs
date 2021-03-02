use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use std::io;

use crossbeam::channel;
use mio::{Poll, Token, Waker};

use crate::socket::{Socket, PeerType, ConnOpts};
use crate::types::FromDaemon as ToService;
use crate::types::ToDaemon as FromService;
use crate::types::Expired;
use crate::state::State;
use crate::daemon::poll;
use crate::timer::{Timers, TimerKind};
use crate::error;


pub fn handle<'a, T>(msg: FromService,
  poll: &Poll,
  timers: &'a mut T,
  token_map: &mut HashMap<Token, Socket>,
  tx_on_write: &channel::Sender<(Token, SocketAddr)>,
  tx_on_close: &channel::Sender<Token>,
  waker: &Arc<Waker>,
  next_conn_id: &mut usize)
  where T: Timers<'a,
    Item=((Token, SocketAddr), TimerKind),
    Expired = Expired<'a, T>> {

  match msg {
    FromService::Connect(io, respond_tx, peer_addr) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, local_addr)) => {
          let conn_opts = ConnOpts::new(token, respond_tx, tx_on_write.clone(), Arc::clone(waker));
          let now = Instant::now();
          let timer_id = (token, peer_addr);
          let state = State::init(now, timer_id, timers, conn_opts);
          conn.send_to(b"hello", peer_addr).ok()
            .or_else(|| { poll::deregister_io(poll, &mut conn); None })
            .map(|_| {
              let socket = Socket::new(conn, local_addr, PeerType::Direct(peer_addr, state));
              token_map.insert(token, socket);
            });
        }
        None => drop(respond_tx)
      }
    }

    FromService::Listen(io, respond_tx) => {
      match poll::register_io(poll, io, next_conn_id) {
        Some((token, mut conn, local_addr)) => {
          let on_close = {
            let tx_on_close = tx_on_close.clone();
            let waker = Arc::clone(waker);
            move || -> io::Result<()> {
              tx_on_close.send(token).map_err(error::cannot_send_to_daemon)?;
              waker.wake().map_err(error::wake_failed)?;
              Ok(())
            }
          };

          respond_tx.send(ToService::Listener(Box::new(on_close)))
            .map_err(|_| poll::deregister_io(poll, &mut conn)).ok()
            .map(|_| {
              let tx_on_write = tx_on_write.clone();
              let waker = Arc::clone(waker);
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
