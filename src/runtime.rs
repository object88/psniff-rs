use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use futures::future::join_all;
use log::{error, info};
use thiserror::Error;
use tokio::{
	runtime,
	signal::ctrl_c,
	sync::broadcast::{self, Receiver},
	task::JoinSet,
};

#[derive(Debug, Error)]
pub enum Error<T: std::error::Error> {
	#[error("")]
	BuildError { inner_error: Box<T> },
}

// pub struct Built {
//   runnable: impl Runnable,
// }

// impl Runnable for Built {
//   #[must_use]
//   #[allow(elided_named_lifetimes,clippy::type_complexity,clippy::type_repetition_in_bounds)]
//   fn run<'life0,'async_trait>(&'life0 mut self,cancel_token:CancellationToken) ->  ::core::pin::Pin<Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send+'async_trait> >where 'life0:'async_trait,Self:'async_trait {
//     self.runnable.run()
//   }
// }
pub trait BlockingRunnable: Send {
	fn run(self: Box<Self>, cancel_rx: Receiver<()>) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait Runnable: Send {
	async fn run(&mut self, cancel_rx: Receiver<()>);
}

// impl<T: Runnable> Runnable for T {
//   async fn run(&mut self, cancel_token: CancellationToken) {
//     self.run(cancel_token);
//   }
// }

pub trait BlockingRunnableBuilder: Send {
	fn build(self: Box<Self>)
	-> Result<Box<dyn BlockingRunnable + Send>, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait RunnableBuilder: Send {
	async fn build(self: Box<Self>) -> Result<Box<dyn Runnable>, Box<dyn std::error::Error>>;
}

pub fn blocking_build(
	buildables: impl IntoIterator<Item = Box<dyn BlockingRunnableBuilder>>,
) -> Result<Vec<Box<dyn BlockingRunnable + Send>>, Box<dyn std::error::Error>> {
	let results = buildables.into_iter().map(|f| f.build());

	let mut v = vec![];
	for r in results {
		match r {
			Ok(r) => v.push(r),
			Err(_e) => {},
		}
	}

	Ok(v)
}

pub async fn build(
	buildables: impl IntoIterator<Item = Box<dyn RunnableBuilder>>,
) -> Result<Vec<Box<dyn Runnable>>, Box<dyn std::error::Error>> {
	let futures = buildables.into_iter().map(|f| f.build());
	let results = join_all(futures).await;

	let mut v = vec![];
	for r in results {
		match r {
			Ok(r) => {
				v.push(r);
			},
			Err(_e) => {},
		}
	}

	Ok(v)
}

pub fn run(
	blocking_runnable_builders: impl IntoIterator<Item = Box<dyn BlockingRunnableBuilder>>,
	runnable_builders: impl IntoIterator<Item = Box<dyn RunnableBuilder>>,
) -> Result<()> {
	// Create runtime for network listened
	let rt = runtime::Builder::new_multi_thread()
		.enable_io()
		.thread_name_fn(|| {
			static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
			let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
			format!("worker-thread-{}", id)
		})
		.build()
		.unwrap();

	rt.block_on(async {
		info!("starting block");

		// Build the sync tasks
		let blocking_runnables = match blocking_build(blocking_runnable_builders) {
			Ok(x) => x,
			Err(_e) => {
				return;
			},
		};

		// Build the async tasks
		let runnables = match build(runnable_builders).await {
			Ok(x) => x,
			Err(_e) => {
				return; // Err(Error::BuildError{ inner_error: e });
			},
		};

		// let cancel_token = CancellationToken::new();
		let (shutdown_tx, _) = broadcast::channel::<()>(1);

		let mut futures = JoinSet::new();

		// Start the sync tasks
		for r in blocking_runnables {
			let rx = shutdown_tx.subscribe();
			futures.spawn_blocking(move || {
				let _ = r.run(rx);
			});
		}

		// Start the async tasks
		for mut r in runnables {
			let rx = shutdown_tx.subscribe();
			futures.spawn(async move {
				r.run(rx).await;
			});
		}

		// Wait for a reason to halt
		tokio::select! {
			_ = ctrl_c() => {
				info!("captured ctrl-c");
			},
			_ = futures.join_next() => {},
		}

		// Shutdown everything
		info!("pre-cancel");
		let _ = shutdown_tx.send(());
		info!("post-cancel");

		while let Some(res) = futures.join_next().await {
			match res {
				Ok(_val) => info!("Task returned."),
				Err(e) => error!("Task failed: {:?}", e),
			}
		}

		info!("ending block");
	});

	Ok(())
}
