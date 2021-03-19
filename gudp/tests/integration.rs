mod harness;
mod hex;

#[test]
/*
LOG Description: A packet with the wrong payload is sent. No connection should be made.
SENT 0001: 0ns - ab cd 12 34 00 00 00 00 00 00 00 00 00 00 00 00 74 65 73 74 20 70 61 79 6c 6f 61 64
*/

fn test_wrong_protocol_id() {
  let mut buf = vec![0u8; 4096];
  let harness = harness::new(8000, 9000);
  let send = hex::decode_unsafe("
    ab cd 12 34 00 00 00 00
    00 00 00 00 74 65 73 74
    20 70 61 79 6c 6f 61 64
  ");

  harness.socket.send(&send).expect("Could not send");
  std::thread::sleep(std::time::Duration::from_millis(5));
  match harness.socket.recv(&mut buf) {
    Err(e) => {
      assert_eq!(e.kind(), std::io::ErrorKind::WouldBlock)
    },
    _ => panic!("Expected WouldBlock"),
  }
}

#[test]
/*
LOG Description: A packet with the right payload is sent. We receive an echo reply
SENT 0001: 0ns - de ad be ef 00 00 00 00 00 00 00 00 00 00 00 00
RECEIVED 0000: 0ns - de ad be ef 00 00 00 00 00 00 00 00 00 00 00 00
*/

fn test_right_protocol_id() {
  let mut buf = vec![0u8; 4096];
  let harness = harness::new(8000, 9000);
  let mut send = gudp::PROTOCOL_ID.to_vec();
  send.extend(hex::decode_unsafe("00 00 00 00 00 00 00 00 00 00 00 00"));
  let mut expected = gudp::PROTOCOL_ID.to_vec();
  expected.extend(hex::decode_unsafe("00 00 00 00 00 00 00 00 00 00 00 00"));

  harness.socket.send(&send).expect("Could not send");
  std::thread::sleep(std::time::Duration::from_millis(1));
  let size = harness.socket.recv(&mut buf).expect("Could not recv");
  assert_eq!(&buf[..size], &expected);
}
