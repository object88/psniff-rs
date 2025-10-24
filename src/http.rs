use std::{net::{IpAddr, SocketAddr}, str::FromStr};

use async_trait::async_trait;
use axum::{routing::get, Router};
// use common::config::Http as HttpConfig;
use log::{info};
use thiserror::Error;
use tokio::{net::TcpListener, sync::broadcast::{self, Receiver}};

use crate::runtime::{Runnable, RunnableBuilder};

#[derive(Debug, Error)]
pub enum Error {
  #[error("")]
  BadAddr,
}

pub struct Builder {
  // cfg: HttpConfig
}

pub struct Http {
  app: Option<Router>,
  listener: Option<TcpListener>,
}

pub fn new(/*cfg: HttpConfig*/) -> Builder {
  Builder{
    // cfg
  }
}

#[async_trait]
impl RunnableBuilder for Builder {
// impl Builder {
  async fn build(self: Box<Self>) -> Result<Box<dyn Runnable + 'static>, Box<dyn std::error::Error>> {
    // let ip_addr = match IpAddr::from_str(&self.cfg.host) {
    let ip_addr = match IpAddr::from_str("127.0.0.1") {
      Ok(x) => x,
      Err(_e) => {
        return Err(Error::BadAddr.into())
      }
    };
    // let addr = SocketAddr::new(ip_addr, self.cfg.port);
    let addr = SocketAddr::new(ip_addr, 3000);

    let app = Router::new().route("/", get(|| async { "Hello, World" }));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    return Ok(Box::new(Http{
      app: Some(app),
      listener: Some(listener),
    }))
  }
}

#[async_trait]
impl Runnable for Http {
  async fn run(&mut self, mut cancel_rx: Receiver<()>) {
    info!("http started");
    
    let app = self.app.take();
    let listener = self.listener.take();

    axum::serve(listener.unwrap(), app.unwrap()).with_graceful_shutdown(async move {
      match cancel_rx.recv().await {
        Ok(_) | Err(broadcast::error::RecvError::Closed) => {
          println!("Received close message");
        }
        Err(broadcast::error::RecvError::Lagged(_)) => {},
      }
      // cancel_rx.recv().await 
    }).await.unwrap();
    info!("http exited");
  }
}
