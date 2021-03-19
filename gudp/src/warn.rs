use crossbeam::channel::SendError;
use log::warn;

pub fn tx_to_write_send_failed<T>(e: SendError<T>) {
 warn!("Channel to signal pending writes failed to send! Socket may be delayed in sending: {}", e)
}
