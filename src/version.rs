pub mod built_info {
  // The file has been placed there by the build script.
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn dump() {
  print!("name: {}\n", built_info::PKG_NAME);
}