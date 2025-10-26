use anyhow::Result;
use clap::Parser;
use psniff_rs::{
	cli::{Cli, Commands, logging},
	devices::{self, list, listen},
	http,
	runtime::{self, BlockingRunnableBuilder, RunnableBuilder},
	version,
};

fn main() -> Result<()> {
	let c = Cli::parse();

	// Set up logging
	logging::init(c.log_level.into());

	match &c.command {
		Some(Commands::List {}) => {
			list()?;
		},
		Some(Commands::Listen(args)) => {
			listen(args.into())?;
		},
		Some(Commands::Run {}) => {
			let http_builder = http::new(/*c.server.api_http*/);
			let mut d = devices::new();

			d = d.set_interface("en0".to_string());

			let blocking_v: Vec<Box<dyn BlockingRunnableBuilder>> = vec![Box::new(d)];

			let v: Vec<Box<dyn RunnableBuilder + 'static>> = vec![Box::new(http_builder)];
			let _ = runtime::run(blocking_v, v);
		},
		Some(Commands::Version) => {
			version::dump();
		},
		None => todo!(),
	}
	Ok(())
}
