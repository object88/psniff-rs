use std::{fmt, future::Future, pin::Pin, task::{Context, Poll}, time::Duration};

use pin_project::pin_project;
use tokio::time::Sleep;
use tower::Service;
use tower_layer::Layer;

#[derive(Clone, Debug)]
pub struct TimeoutLayer {
  timeout: Duration,
}

impl TimeoutLayer {
  pub const fn new(timeout: Duration) -> Self {
    TimeoutLayer{ timeout }
  }
}

impl<S> Layer<S> for TimeoutLayer {
  type Service = Timeout<S>;

  fn layer(&self, service: S) -> Self::Service {
    Timeout::new(service, self.timeout)
  }
}

#[derive(Debug, Clone)]
pub struct Timeout<S> {
  inner: S,
  timeout: Duration,
}

impl<S> Timeout<S> {
  fn new(inner: S, timeout: Duration) -> Self {
    Timeout{ inner, timeout }
  }
}

impl<S, Request> Service<Request> for Timeout<S> where S: Service<Request>, S::Error: Into<BoxError> {
  type Response = S::Response;

  type Error = BoxError;

  type Future = ResponseFuture<S::Future>;

  fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_ready(cx).map_err(Into::into)
  }

  fn call(&mut self, req: Request) -> Self::Future {
    let response_future = self.inner.call(req);
    let sleep = tokio::time::sleep(self.timeout);

    ResponseFuture {
      response_future,
      sleep,
    }
  }
}

#[pin_project]
pub struct ResponseFuture<F> {
  #[pin]
  response_future: F,

  #[pin]
  sleep: Sleep,
}

impl<F, Response, Error> Future for ResponseFuture<F> where F: Future<Output = Result<Response, Error>>, Error: Into<BoxError> {
  type Output = Result<Response, BoxError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.project();

    match this.response_future.poll(cx) {
      Poll::Ready(result) => {
        return Poll::Ready(result.map_err(Into::into));
      },
      Poll::Pending => {},
    }

    match this.sleep.poll(cx) {
      Poll::Ready(()) => {
        return Poll::Ready(Err(Box::new(TimeoutError(()))));
      },
      Poll::Pending => {},
    }

    Poll::Pending
  }
}

#[derive(Debug, Default)]
struct TimeoutError(());

impl fmt::Display for TimeoutError {
  fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
    todo!()
  }
}

impl std::error::Error for TimeoutError {}

type BoxError = Box<dyn std::error::Error + Send + Sync>;