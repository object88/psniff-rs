use anyhow::Result;
use clap::Parser;
use psniff_rs::{cli::{Cli, Commands, logging}, devices::{list, listen}, version};

fn main() -> Result<()> {
  let c = Cli::parse();

  logging::init(c.log_level.into());

  match &c.command {
    Some(Commands::List {}) => {
      list()?;
    },
    Some(Commands::Listen(args)) => {
      listen(args.into())?;
    },
    Some(Commands::Version) => {
      version::dump();
    }
    None => todo!(),
  }
  Ok(())
}
