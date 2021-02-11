use std::collections::hash_map::OccupiedEntry;
use std::sync::Arc;

use mio::{Poll, Token};
use mio::net::UdpSocket as MioUdpSocket;

use bring::Bring;

use crate::types::FromDaemon as ToService;
use crate::types::READ_BUFFER_TAG;
use crate::state::{State, FSM};
use crate::sync::CondMutexGuard;

type StateEntry<'a> = OccupiedEntry<'a, Token, (State, MioUdpSocket)>;

// Only call when you're sure status.is_closed() is true!
// Otherwise notified readers might sleep again. See the caveat note below
fn close_remote_socket<'a>(
  poll: &'a Poll,
  socket: &'a mut MioUdpSocket,
  cond_lock: CondMutexGuard<Bring, READ_BUFFER_TAG>
) {
  cond_lock.notify_all();
  drop(cond_lock);
  poll.registry().deregister(socket).expect("Could not deregister");
}

pub fn handle(mut entry: StateEntry, poll: &Poll) {
  let (ref mut state, ref mut socket) = entry.get_mut();
  let (ref buf_read, ref _buf_write, ref status) = *state.shared;

  let mut buf = buf_read.lock().expect("Could not acquire unpoisoned read lock");

  // NOTE: This status check must be done while holding the readlock to ensure no races occur of the form:
  // time0|thread0: client acquires the readlock.
  // time1|thread0: client observes an open status and no pending reads...
  // time2|thread1: open status changes to closed...
  // time3|thread1: this fn observes the closed status and signals the condvar to wake all _current_ sleepers
  // time4|thread0: ... and then client sleeps, oblivious to the change in status and condvar signal
  // Since after this notify_all, no future notifications are coming, client would sleep forever!
  // The solution is to prevent client0 from acquiring the readlock until this fn observes the closed status.
  // That means when the client DOES acquire the readlock, it will observe a closed status and not sleep.
  // Alternatively, if the client acquired the readlock first, it will sleep on the condvar before this fn can notify.
  // Thus ensuring it will hear the notification to wake up and observe the closed status.
  if status.is_closed() {
    close_remote_socket(poll, socket, buf);
    entry.remove();
    return;
  }

  match socket.recv(&mut state.buf_local) {
    Ok(size) => {
      buf.push_back(&state.buf_local[..size]).map(|_| buf.notify_one());
      drop(buf);

      // If we were listening, we now know we have a live connection to accept
      if let FSM::Listen{ tx } = &state.fsm {
        tx.send(
          ToService::Connection(Arc::clone(&state.shared))
        ).expect("Could not finish listening with connection state");
        state.fsm = FSM::Connected;
      }
    },

    // TODO: If a fatal connection error happens, we must call status.set_remote_drop(),
    // and notify all sleepers BEFORE dropping the readlock. It is important that we synchronize with the readlock
    // for the reasons explained above. Then we can deregister as usual
    Err(e) => {
      if e.kind() == std::io::ErrorKind::WouldBlock {} // This is fine for mio
      else { // TODO: Handle errors explicitly. Set remote drop flags based on errorkind
        status.set_remote_drop();
        close_remote_socket(poll, socket, buf);
        entry.remove();
      }
    }
  }
}
