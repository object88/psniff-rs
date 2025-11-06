use serde::Deserialize;

pub struct ListConfig {}

pub struct ListenConfig {
	pub interfaces: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Http {
	pub host: String,
	pub port: u16,
}

pub struct RunConfig {
	pub api_http: Http,
}