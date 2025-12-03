use axum::{Router, routing::MethodRouter};

use crate::state::appstate::State;

pub struct Route<S>
where
	S: State + 'static,
{
	router: Router<S>,
}

pub fn new<S>() -> Result<Route<S>, Box<dyn std::error::Error>>
where
	S: State + 'static,
{
	let r = Router::new();
	Ok(Route::<S> { router: r })
}

impl<S> Route<S>
where
	S: State + 'static,
{
	pub fn add<H>(mut self, path: &str, f: H) -> Self
	where
		H: Into<MethodRouter<S>>,
	{
		self.router = self.router.route(path, f.into());
		self
	}
}

impl<S> From<Route<S>> for Router<S>
where
	S: State,
{
	fn from(val: Route<S>) -> Self {
		val.router
	}
}
