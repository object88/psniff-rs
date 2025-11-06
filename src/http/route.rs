use axum::{routing::MethodRouter, Router};

pub trait State: Clone + Default + Send + Sync {}

pub struct Route<S> where S: State + 'static {
  router: Router<S>,
}

pub fn new<S>() -> Result<Route<S>, Box<dyn std::error::Error>> where S: State + 'static {
  let r = Router::new();
  Ok(Route::<S>{
    router: r,
  })
}

impl<'a, S> Route<S> where S: State + 'static {
  pub fn add<H>(mut self, path: &str, f: H) -> Self 
    where
      H: Into<MethodRouter<S>>,
  {
    self.router = self.router.route(path, f.into());
    self
  }
}

impl<S> Into<Router<S>> for Route<S> where S: State {
  fn into(self) -> Router<S> {
    self.router
  }
}