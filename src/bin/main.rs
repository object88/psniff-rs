use anyhow::Result;
use axum::{extract::State, routing::get};
use clap::Parser;
use psniff_rs::{
	appstate::{self, AppState}, cli::{logging, Cli, Commands}, config::RunConfig, devices::{self, list, listen}, http::{route, service as http_s}, packet_listeners::listener, runtime::{self, BlockingRunnableBuilder, RunnableBuilder}, version
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
		Some(Commands::Run(args)) => {
			let rc: RunConfig = args.into();

			// Construct the state
			let app_state = appstate::new::<'static>();

			let (sender, receiver) = tokio::sync::mpsc::channel::<devices::MovingPacket>(1024);

			// Create guard at the start of your program (only when feature is enabled)
			#[cfg(feature = "channels-console")]
			let _guard = channels_console::ChannelsGuard::new();
		
			#[cfg(feature = "channels-console")]
			let (sender, receiver) = channels_console::instrument!((sender, receiver), label = "packet-queue");

			// Construct the packet listener builder
			let listener_builder = listener::new()
				.set_receiver(receiver);

			// Construct the HTTP routes and builder
			let route = match route::new() {
				Ok(r) => r,
				Err(e) => {
					return Err(anyhow::anyhow!(e.to_string()));
				}
			}
				.add("/status/ready", get(|| async { "wat" }))
				.add("/status", get(|State(_state): State<AppState<'static, ()>>| async { "yup" }));

			let http_builder = http_s::new::<AppState<'static,()>>(rc.api_http)
				.set_routes(route)
				.set_state(app_state);

			// Construct the network device listener
			let mut d = devices::new();

			d = d.set_interface("en0".to_string())
				.set_typed_sender(devices::Matcher::IPv4_TCP, sender);

			let blocking_v: Vec<Box<dyn BlockingRunnableBuilder>> = vec![Box::new(d)];

			let v: Vec<Box<dyn RunnableBuilder + 'static>> = vec![
				Box::new(http_builder),
				Box::new(listener_builder),
			];
			let _ = runtime::run(blocking_v, v);
		},
		Some(Commands::Version) => {
			version::dump();
		},
		None => todo!(),
	}
	Ok(())
}
