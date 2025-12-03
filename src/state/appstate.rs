use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex},
};

use crate::{
	devices::Matcher,
	state::{interface::Interface, packet_count::PacketCount},
};

pub trait State: Clone + Default + Send + Sync {}

#[derive(Default)]
pub struct AppState {
	pub interfaces: Arc<Mutex<HashSet<Arc<Interface>>>>,
	pub packet_counts: HashMap<Matcher, Arc<Mutex<PacketCount>>>,
}

pub fn new() -> AppState {
	AppState {
		interfaces: Arc::new(Mutex::new(HashSet::new())),
		packet_counts: HashMap::new(),
	}
}

impl Clone for AppState {
	fn clone(&self) -> Self {
		Self {
			interfaces: self.interfaces.clone(),
			packet_counts: self.packet_counts.clone(),
		}
	}
}

impl State for AppState {}
