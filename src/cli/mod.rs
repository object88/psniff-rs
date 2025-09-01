pub mod args;
pub mod logging;

use clap::{Parser, Subcommand};

use crate::{cli::args::ArgLevelFilter, config::ListenConfig};

#[derive(Parser)]
#[command(arg_required_else_help = true)]
pub struct Cli {

  #[arg(default_value = "warn", long)]
  pub log_level: ArgLevelFilter,

  #[command(subcommand)]
  pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
  /// List interfaces
  List {

  },

  /// Listen to one or more interfaces
  Listen(ArgsListen),

  Version,
}

#[derive(Parser)]
pub struct ArgsListen {
  #[arg(group = "interfaces_group", long)]
  pub interfaces: Option<Vec<String>>,

  #[arg(default_value_t = false, group = "interfaces_group", long)]
  pub all_interfaces: bool

}

impl Into<ListenConfig> for &ArgsListen {
  fn into(self) -> ListenConfig {
    ListenConfig { interfaces: self.interfaces.clone() }
  }
}