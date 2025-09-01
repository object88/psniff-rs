use std::io::stderr;

use log::LevelFilter;
use structured_logger::{json::new_writer, Builder};

pub fn init(level: LevelFilter) {
	// Set up logging
	Builder::with_level(level.as_str())
			.with_target_writer("*", new_writer(stderr()))
			.init();
}