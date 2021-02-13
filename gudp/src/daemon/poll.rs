use mio::Poll;
use mio::net::UdpSocket as MioUdpSocket;

use bring::Bring;
use cond_mutex::CondMutexGuard;

use crate::types::READ_BUFFER_TAG;

// Only call when you've ensured status.is_closed() is true!
// Otherwise notified readers might sleep again.
// See notes in read_event and write_event
pub fn close_remote_socket<'a>(
  poll: &'a Poll,
  socket: &'a mut MioUdpSocket,
  cond_lock: CondMutexGuard<Bring, READ_BUFFER_TAG>
) {
  cond_lock.notify_all();
  drop(cond_lock);
  poll.registry().deregister(socket).expect("Could not deregister");
}
