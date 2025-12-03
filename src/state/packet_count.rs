#[derive(Clone, Default, Eq, PartialEq)]
pub struct PacketCount {
	pub total: u32,
	pub os_dropped: u32,
	pub if_dropped: u32,
}
