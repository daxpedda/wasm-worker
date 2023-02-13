mod builder;
mod common;
mod context;
mod handle;
mod worker_url;

use std::future::Future;

pub use self::builder::{ModuleSupportError, WorkerBuilder};
use self::common::{Closure, WorkerOrContext, EXPORTS};
pub use self::common::{Tls, TransferError};
pub use self::context::WorkerContext;
pub use self::handle::{DestroyError, WorkerHandle, WorkerHandleRef};
pub use self::worker_url::{WorkerUrl, WorkerUrlFormat};

pub fn spawn<F>(f: F) -> WorkerHandle
where
	F: 'static + FnOnce(WorkerContext) + Send,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

pub fn spawn_async<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = ()>,
{
	WorkerBuilder::new().unwrap().spawn_async(f)
}
