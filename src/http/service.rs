use std::{
	mem,
	net::{IpAddr, SocketAddr},
	str::FromStr,
	time::Duration,
};

use async_trait::async_trait;
use axum::{BoxError, Router, error_handling::HandleErrorLayer, http::StatusCode};
use log::info;
use thiserror::Error;
use tokio::{
	net::TcpListener,
	sync::broadcast::{self, Receiver},
};
use tower::ServiceBuilder;

use crate::{
	config::Http as HttpConfig,
	http::{middleware, route::Route},
	runtime::{Runnable, RunnableBuilder},
	state::appstate::State,
};

#[derive(Debug, Error)]
pub enum Error {
	#[error("bad address")]
	BadAddr,

	#[error("no routes on http service")]
	NoRouting,

	#[error("no state on http service")]
	NoState,
}

pub struct Builder<S>
where
	S: State + 'static,
{
	cfg: HttpConfig,
	routes: Option<Route<S>>,
	state: Option<S>,
}

pub struct Http<S>
where
	S: State,
{
	app: Router<S>,
	listener: Option<TcpListener>,
	s: S,
}

pub fn new<S>(cfg: HttpConfig) -> Builder<S>
where
	S: State,
{
	Builder {
		cfg,
		routes: None,
		state: None,
	}
}

impl<S> Builder<S>
where
	S: State,
{
	pub fn set_routes(mut self, routes: Route<S>) -> Self {
		self.routes = Some(routes);
		self
	}

	pub fn set_state(mut self, state: S) -> Self {
		self.state = Some(state);
		self
	}
}

#[async_trait]
impl<S> RunnableBuilder for Builder<S>
where
	S: State,
{
	async fn build(
		self: Box<Self>,
	) -> Result<Box<dyn Runnable + 'static>, Box<dyn std::error::Error>> {
		let ip_addr = match IpAddr::from_str(&self.cfg.host) {
			Ok(x) => x,
			Err(_e) => return Err(Error::BadAddr.into()),
		};
		let addr = SocketAddr::new(ip_addr, self.cfg.port);

		let r = match self.routes {
			Some(r) => r,
			None => return Err(Error::NoRouting.into()),
		};

		let s = match self.state {
			Some(s) => s,
			None => return Err(Error::NoState.into()),
		};

		let r0: Router<S> = <Route<S> as Into<axum::Router<S>>>::into(r).layer(
			ServiceBuilder::new()
				.layer(HandleErrorLayer::new(handle_error))
				.layer(middleware::TimeoutLayer::new(Duration::from_millis(250))),
		);

		let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
		return Ok(Box::new(Http {
			app: r0,
			listener: Some(listener),
			s,
		}));
	}
}

#[async_trait]
impl<S> Runnable for Http<S>
where
	S: State + 'static,
{
	async fn run(&mut self, mut cancel_rx: Receiver<()>) {
		info!("http started");

		let app = mem::take(&mut self.app);
		let listener = self.listener.take();
		let state = mem::take(&mut self.s);

		axum::serve(listener.unwrap(), app.with_state(state))
			.with_graceful_shutdown(async move {
				match cancel_rx.recv().await {
					Ok(_) | Err(broadcast::error::RecvError::Closed) => {
						info!("Received close message");
					},
					Err(broadcast::error::RecvError::Lagged(_)) => {},
				}
			})
			.await
			.unwrap();
		info!("http exited");
	}
}

async fn handle_error(_err: BoxError) -> (StatusCode, String) {
	(StatusCode::BAD_REQUEST, "error".to_string())
}
