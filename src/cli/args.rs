use clap::{builder::PossibleValue, ValueEnum};
// use common::config::Config;
// use dirs::{config_local_dir, home_dir};
use log::LevelFilter;

/// ArgLevelFilter is a newtype for LevelFilter, so that ValueEnum can be
/// implemented
#[derive(Clone)]
pub struct ArgLevelFilter(pub LevelFilter);

impl Into<LevelFilter> for ArgLevelFilter {
  fn into(self) -> LevelFilter {
    self.0
  }
}

// ValueEnum is necessary for EnumValueParser
impl ValueEnum for ArgLevelFilter {
  fn value_variants<'a>() -> &'a [Self] {
    &[
      ArgLevelFilter(LevelFilter::Off),
      ArgLevelFilter(LevelFilter::Error),
      ArgLevelFilter(LevelFilter::Warn),
      ArgLevelFilter(LevelFilter::Info),
      ArgLevelFilter(LevelFilter::Debug),
      ArgLevelFilter(LevelFilter::Trace)
    ]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(PossibleValue::new(Into::<LevelFilter>::into(self.0).to_string().to_lowercase()))
  }
}

// // pub struct Args<'a>{
// pub struct Args {
//   c: Command,

//   // cfg: &'a mut Config,

//   // config_path: String
//   pub interface: Option<String>,

//   pub log_level: LevelFilter,
// }

// // impl Args<'_> {
// impl Args {
//   pub fn new(/*cfg: &mut Config*/) -> Result<Args, ArgsError> {
//     // let default_config_dir: PathBuf = match config_local_dir() {
//     //   Some(mut x) => {
//     //     x.push("");
//     //     x
//     //     // let mut x0 = x.clone();
//     //     // x0.push("");
//     //     // x0.to_str().unwrap()
//     //   },
//     //   None => PathBuf::new()
//     // };

//     // let default_config_dir: String = default_config_dir.to_string_lossy().to_string();
//     // let default_log_level: String = cfg.server.log_level.to_string().to_lowercase();
//     let default_log_level = "warn".to_string();

//     let cmd = Command::new(env!("CARGO_CRATE_NAME"))
//       .bin_name("psniff")
//       .subcommand_required(true)
//       .long_version("...")
//       .subcommand(Command::new("listen")
//         .arg(Arg::new("allinterfaces")
//           .long("all-interfaces"))
//         .arg(Arg::new("interfaces")
//           .long("interfaces"))
//         .group(ArgGroup::new("interfaces_group")
//           .args(&["allinterfaces", "interfaces"])
//           .required(true)
//           .multiple(false)))
//       // .arg(arg!(--config <FILE> "").default_value(default_config_dir))
//       .arg(Arg::new("loglevel")
//         .long("log-level")
//         .required(false)
//         .default_value(default_log_level)
//         .help("Set the loglevel")
//         .long_help("Set the loglevel. 'trace' is the most verbose and 'off' the least verbose")
//         .value_parser(EnumValueParser::<ArgLevelFilter>::new())
//       );

//     let a: Args = Args{
//       c: cmd,
//       // cfg,
//       // config_path: default_config_dir,
//       log_level: LevelFilter::Warn,
//       interface: None,
//     };
//     Ok(a)
//   }

//   pub fn parse(mut self) -> Result<(), ArgsError> {
//     let x = match self.c.try_get_matches() {
//       Ok(x) => x,
//       Err(e) => {
//         match e.kind() {
//           // clap::error::ErrorKind::InvalidValue => todo!(),
//           // clap::error::ErrorKind::UnknownArgument => todo!(),
//           // clap::error::ErrorKind::InvalidSubcommand => todo!(),
//           // clap::error::ErrorKind::NoEquals => todo!(),
//           // clap::error::ErrorKind::ValueValidation => todo!(),
//           // clap::error::ErrorKind::TooManyValues => todo!(),
//           // clap::error::ErrorKind::TooFewValues => todo!(),
//           // clap::error::ErrorKind::WrongNumberOfValues => todo!(),
//           // clap::error::ErrorKind::ArgumentConflict => todo!(),
//           // clap::error::ErrorKind::MissingRequiredArgument => todo!(),
//           // clap::error::ErrorKind::MissingSubcommand => todo!(),
//           // clap::error::ErrorKind::InvalidUtf8 => todo!(),
//           ErrorKind::DisplayHelp => {
//             // let help = self.c.render_help();
//             // print!("{}\n", help);
//             return Err(ArgsError::Done);
//           },
//           // clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => todo!(),
//           ErrorKind::DisplayVersion => {
//             // let version= self.c.render_version();
//             // print!("{}\n", version);
//             return Err(ArgsError::Done);
//           },
//           // clap::error::ErrorKind::Io => todo!(),
//           // clap::error::ErrorKind::Format => todo!(),
//           _ => {
//             return Err(ArgsError::UnmatchedCommand(e));        
//           },
//         }
//       }
//     };

//     println!("{:?}", x);

//     match x.subcommand() {
//       Some(a) => {
//         let log_level_value: &LevelFilter = &a.1.get_one::<ArgLevelFilter>("loglevel").unwrap().0;

//         self.log_level = *log_level_value;

//       },
//       None => {

//       }
//     }

//     logging::init(self.log_level);

//     Ok(())


//     // match self.c.try_get_matches().unwrap_or_else(|e| {
//     //   return Err(ArgsError::UnmatchedCommand(e));
//     // }).subcommand() {
//     //   a => {
//     //     let log_level_value: &LevelFilter = &a.get_one::<ArgLevelFilter>("loglevel").unwrap().0;
        
//     //     // self.cfg.server.kube_config.path = a.get_one::<PathBuf>("kubeconfig").unwrap().to_path_buf();

//     //     self.log_level = *log_level_value;

//     //   },
//     // }
//   }
// }

// #[derive(Debug, Error)]
// pub enum ArgsError {
//   #[error("")]
//   Done,

//   #[error("")]
//   NoConfigDirectory,

//   #[error("Unknown command: {_0}")]
//   UnmatchedCommand(ClapError),
// }