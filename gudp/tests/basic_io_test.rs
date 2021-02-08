mod basic_io;

use basic_io::TestIO;
use gudp::Connection;

#[test]
fn it_sends() {
  let test_io = TestIO::default();
  {
    let mut test_io = test_io.get_inner();
    test_io.recv_from.push(b"hello from test".to_vec());
  }

  let conn = Connection::from_parts(test_io.clone(), ());
  let _send_result = conn.send(b"hello to test");
  let mut recv_buf = [0u8; 1000];
  let _recv_result = conn.recv(&mut recv_buf);

  {
    let test_io = test_io.get_inner();
    assert_eq!(test_io.sent_to[0], b"hello to test");
    assert_eq!(&recv_buf[..15], b"hello from test");
  }
}
