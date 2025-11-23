use anyhow::Result;
use axum::{extract::State, routing::get};
use clap::Parser;
use psniff_rs::{
	appstate::{self, AppState},
	cli::{Cli, Commands, logging},
	config::RunConfig,
	devices::{self, list, listen},
	http::{route, service as http_s},
	packet_listeners::{ipv4_tcp_listener, ipv4_udp_listener},
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
		Some(Commands::Run(args)) => {
			let rc: RunConfig = args.into();

			// Construct the state
			let app_state = appstate::new::<'static>();

			let (ipv4_tcp_sender, ipv4_tcp_receiver) = tokio::sync::mpsc::channel::<devices::MovingPacket>(1024);
			let (ipv4_udp_sender, ipv4_udp_receiver) = tokio::sync::mpsc::channel::<devices::MovingPacket>(1024);

			// Create guard at the start of your program (only when feature is enabled)
			#[cfg(feature = "channels-console")]
			let _guard = channels_console::ChannelsGuard::new();
		
			#[cfg(feature = "channels-console")]
			let (sender, receiver) = channels_console::instrument!((sender, receiver), label = "packet-queue-ipv4-tcp");

			#[cfg(feature = "channels-console")]
			let (ipv4_udp_sender, ipv4_udp_receiver) = channels_console::instrument!((ipv4_udp_sender, ipv4_udp_receiver), label = "packet-queue-ipv4-udp");

			// Construct the packet listener builders
			let ipv4_tcp_listener_builder = ipv4_tcp_listener::new()
				.set_receiver(ipv4_tcp_receiver);

			let ipv4_udp_listener_builder = ipv4_udp_listener::new()
				.set_receiver(ipv4_udp_receiver);

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
			let d = devices::new()
				.set_interface("en0".to_string())
				.set_typed_sender(devices::Matcher::IPv4_TCP, ipv4_tcp_sender)
				.set_typed_sender(devices::Matcher::IPv4_UDP, ipv4_udp_sender);

			let blocking_v: Vec<Box<dyn BlockingRunnableBuilder>> = vec![Box::new(d)];

			let v: Vec<Box<dyn RunnableBuilder + 'static>> = vec![
				Box::new(http_builder),
				Box::new(ipv4_tcp_listener_builder),
				Box::new(ipv4_udp_listener_builder),
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
