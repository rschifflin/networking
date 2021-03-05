use std::time::Duration;

pub struct Output {
  pub actions: output::Actions,
  pub fields: output::Fields,
}

pub mod output {
  pub struct Actions {
    pub send: bool,
    pub tick: bool,
  }

  pub struct Fields {
    pub send_string: Option<String>
  }
}

pub struct Input<'a> {
  pub sent: &'a Vec<(Duration, Vec<u8>)>,
  pub received: &'a Vec<(Duration, Vec<u8>)>,

  // retained ui strings here
  pub fields: input::Fields<'a>
}

pub mod input {
  pub struct Fields<'a> {
    pub send_string: &'a str,
  }
}
