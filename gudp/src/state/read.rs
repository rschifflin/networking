use std::net::SocketAddr;
use std::sync::Arc;
use std::io;

use bring::bounded::Bring;

use crate::error;
use crate::types::READ_BUFFER_TAG;
use crate::types::FromDaemon as ToService;
use crate::state::{State, Status, Closer, FSM};

impl State {
  pub fn read(&mut self, peer_addr: SocketAddr, buf_local: &mut [u8], buf_size: usize) -> Result<(), Closer> {
    let (ref buf_read, ref buf_write, ref status) = *self.shared;

    // TODO: Should we handle a poisoned lock state here? IE if a thread with a connection panics,
    // what should the daemon do about it? Just close the connection?
    // Likely the client should panic on poison, and the daemon should recover the lock and close the conn on poison
    // For now just panic
    let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

    // NOTE: This status check must be done while holding the readlock to ensure no races occur of the form:
    // time0|thread0: client acquires the readlock.
    // time1|thread0: client observes an open status and no pending reads...
    // time2|thread1: open status changes to closed...
    // time3|thread1: this fn observes the closed status and signals the condvar to wake all _current_ sleepers
    // time4|thread0: ... and then client sleeps, oblivious to the change in status and condvar signal
    // Since after this notify_all, no future notifications are coming, client would sleep forever!
    // The solution is to prevent the client from acquiring the readlock until this fn observes the closed status.
    // That means when the client DOES acquire the readlock, it will observe a closed status and not sleep.
    // Alternatively, if the client acquired the readlock first, it will sleep on the condvar before this fn can notify.
    // Thus ensuring it will hear the notification to wake up and observe the closed status.
    if let (Some(closer)) = status.test_closed() {
      buf.notify_all();
      return Err(closer);
    }

    match &self.fsm {
      FSM::Handshaking { token, tx_to_service, tx_on_write, waker } => {
        let on_write = {
          let token = *token;
          let tx_on_write = tx_on_write.clone();
          let waker = Arc::clone(waker);

          move |size| -> io::Result<usize> {
            tx_on_write.send((token, peer_addr)).map_err(error::cannot_send_to_daemon)?;
            waker.wake().map_err(error::wake_failed)?;
            Ok(size)
          }
        };

        match tx_to_service.send(ToService::Connection(Box::new(on_write), Arc::clone(&self.shared))) {
          Ok(_) => {
            buf.push_back(&mut buf_local[..buf_size]).map(|_| buf.notify_one());
            self.fsm = FSM::Connected;
            Ok(())
          },
          Err(_) => {
            // Failed to create the connection. Deregister
            // NOTE: Setting status is technically not necessary, there is no clientside to observe this
            status.set_client_hup();
            buf.notify_all();
            Err(Closer::Application)
          }
        }
      },
      FSM::Connected => {
        buf.push_back(&mut buf_local[..buf_size]).map(|_| buf.notify_one());
        Ok(())
      }
    }
  }
}
