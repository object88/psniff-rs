use std::io::stderr;

use log::LevelFilter;
use structured_logger::{Builder, json::new_writer};

pub fn init(level: LevelFilter) {
	// Set up logging
	Builder::with_level(level.as_str())
		// .with_target_writer(targets, writer)
		.with_target_writer("*", new_writer(stderr()))
		.init();
}
