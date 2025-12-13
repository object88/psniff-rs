use tun::{AbstractDevice, Configuration, Device};

pub fn setup() -> (Device, String) {
	let mut config = Configuration::default();
	config
		.address((10, 0, 0, 1))
		.netmask((255, 255, 255, 0))
		.destination((10, 0, 0, 9))
		.up();

	#[cfg(target_os = "macos")]
	config.layer(tun::Layer::L3);

	let dev = tun::create(&config).unwrap();
	let name = dev.tun_name().unwrap();
	println!("Created TUN device with name {name}");

	(dev, name)
}
