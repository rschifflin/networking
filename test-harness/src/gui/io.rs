pub mod input {
  use imgui::ImString;
  pub struct Fields {
    pub send_string: ImString,
    pub log_string: ImString,
    pub tick_amount: i32
  }
}

pub mod output {
  pub struct Actions {
    pub send: bool,
    pub tick: bool,
    pub log: bool
  }
}
