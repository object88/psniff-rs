use std::{marker::PhantomData, sync::atomic::{AtomicU8, Ordering}};

use crate::http::route::State;

pub struct AppState<'a, T>  {
  flags: AtomicU8,
  _marker: PhantomData<&'a T>
}

impl<'a, T> State for AppState<'a, T> where T: Send + Sync {}

pub fn new<'a, T>() -> AppState<'a, T>  {
  AppState::<'a, T> {
    flags: AtomicU8::new(0),
    _marker: PhantomData{},
  }
}

impl<'a, T>  Default for AppState<'a, T>  {
  fn default() -> Self {
    Self { 
      flags: Default::default(),
      _marker: PhantomData{},
    }
  }
}

impl<'a, T>  Clone for AppState<'a, T>  {
	fn clone(&self) -> Self {
		Self {
      flags: AtomicU8::new(self.flags.load(Ordering::Acquire)),
      _marker: PhantomData{},
    }
	}
}
