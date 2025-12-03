#[derive(Clone, Eq, PartialEq)]
pub struct PacketCount {
  pub total: u32,
  pub os_dropped: u32,
  pub if_dropped: u32,
}

impl Default for PacketCount {
  fn default() -> Self {
    Self { total: Default::default(), os_dropped: Default::default(), if_dropped: Default::default() }
  }
}
