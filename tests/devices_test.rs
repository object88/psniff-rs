use pcap::Device;

mod common;

// use crate::devices::list;

#[test]
fn test_tun_creation() {
	let (_dev, name) = common::setup();

	let list = Device::list().unwrap();
	assert!(list.iter().any(|d| d.name == name));
}

#[test]
fn test_loop() {}
