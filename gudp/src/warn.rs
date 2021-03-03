use crossbeam::channel::SendError;
use log::warn;

pub fn tx_to_write_send_failed<T>(e: SendError<T>) {
 warn!("Channel to signal pending writes failed to send! Socket may be delayed in sending: {}", e)
}

pub fn prepare_heartbeat_failed() {
 warn!("Could not prepare heartbeat to send: Destination buffer full or source buffer too small. Dropping heartbeat; this may lead to a premature timeout!")
}
