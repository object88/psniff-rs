pub mod args;
pub mod logging;

use clap::{Parser, Subcommand};

use crate::{
	cli::args::ArgLevelFilter,
	config::{Http, ListenConfig, RunConfig},
};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Cli {
	#[arg(default_value = "info", long)]
	pub log_level: ArgLevelFilter,

	#[command(subcommand)]
	pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
	/// List interfaces
	List {},

	/// Listen to one or more interfaces
	Listen(ArgsListen),

	/// Run
	Run(ArgsRun),

	Version,
}

#[derive(Parser)]
pub struct ArgsListen {
	#[arg(group = "interfaces_group", long)]
	pub interfaces: Option<Vec<String>>,

	#[arg(default_value_t = false, group = "interfaces_group", long)]
	pub all_interfaces: bool,
}

impl From<&ArgsListen> for ListenConfig {
	fn from(val: &ArgsListen) -> Self {
		ListenConfig {
			interfaces: val.interfaces.clone(),
		}
	}
}

#[derive(Parser)]
pub struct ArgsRun {
	#[arg(default_value = "127.0.0.1")]
	pub host: String,

	#[arg(default_value_t = 3000)]
	pub port: u16,
}

impl From<&ArgsRun> for RunConfig {
	fn from(value: &ArgsRun) -> Self {
		RunConfig {
			api_http: Http {
				host: value.host.clone(),
				port: value.port,
			},
		}
	}
}
