pub mod input {
  use imgui::ImString;
  pub struct Fields {
    pub send_hexstring: ImString,
    pub log_string: ImString,
    pub elapsed_string: ImString,
    pub tick_amount: i32,

    // Header
    pub magic_bytes_hexstring: ImString,
    pub local_sequence_no_numstring: ImString,
    pub remote_sequence_no_numstring: ImString,
    pub remote_sequence_tail_bitstring: ImString,

    // Payload
    pub payload_string: ImString
  }

  impl Default for Fields {
    fn default() -> Fields {
      let mut local_sequence_no_numstring = ImString::with_capacity(128);
      local_sequence_no_numstring.push_str("0");
      let mut remote_sequence_no_numstring = ImString::with_capacity(128);
      remote_sequence_no_numstring.push_str("0");

      Fields {
        send_hexstring: ImString::with_capacity(128),
        log_string: ImString::with_capacity(4096),
        tick_amount: 1000,
        elapsed_string: ImString::new("0s"),
        magic_bytes_hexstring: ImString::new("deadbeef"),
        local_sequence_no_numstring,
        remote_sequence_no_numstring,
        remote_sequence_tail_bitstring: ImString::new("00000000 00000000 00000000 00000000"),
        payload_string: ImString::with_capacity(128),
      }
    }
  }
}

pub mod output {
  #[derive(Default)]
  pub struct Actions {
    pub send: bool,
    pub tick: bool,
    pub log: bool
  }
}
