use std::{
	borrow::Borrow,
	hash::{Hash, Hasher},
	sync::Mutex,
};

use crate::state::packet_count::PacketCount;

pub struct Interface {
	name: String,
	counts: Mutex<PacketCount>,
	watching: bool,
}

impl Interface {
	pub fn new(name: String) -> Interface {
		Interface {
			name,
			counts: Default::default(),
			watching: true,
		}
	}

	pub fn count(&self) -> u32 {
		self.counts.lock().unwrap().total
	}

	pub fn update_counts(&self, total: u32, os_dropped: u32, if_dropped: u32) {
		let mut counts = self.counts.lock().unwrap();
		counts.total = total;
		counts.os_dropped = os_dropped;
		counts.if_dropped = if_dropped;
	}
}

impl Borrow<str> for Interface {
	fn borrow(&self) -> &str {
		&self.name
	}
}

impl Clone for Interface {
	fn clone(&self) -> Self {
		Self {
			name: self.name.clone(),
			counts: Mutex::new(self.counts.lock().unwrap().clone()),
			watching: self.watching.clone(),
		}
	}
}

impl Eq for Interface {}

impl Hash for Interface {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.name.hash(state);
	}
}

impl PartialEq for Interface {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

#[cfg(test)]
mod tests {
	use crate::state::interface::Interface;

	#[test]
	fn test_update_count() {
		let iface = Interface::new("foo".to_string());
		iface.update_counts(13, 3, 1);

		assert_eq!(13, iface.count());
	}
}
