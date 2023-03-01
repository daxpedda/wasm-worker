mod builder;
mod context;
mod support;
mod url;
mod worker;

use std::future::Future;

pub use self::builder::WorkerBuilder;
pub use self::context::WorkerContext;
pub use self::support::{has_async_support, AsyncSupportError, AsyncSupportFuture};
pub use self::url::{ModuleSupportError, WorkerUrl};
pub use self::worker::{DestroyError, Worker, WorkerRef};

#[track_caller]
pub fn spawn<F>(f: F) -> Worker
where
	F: 'static + FnOnce(WorkerContext) + Send,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

#[track_caller]
pub fn spawn_async<F1, F2>(f: F1) -> Worker
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = ()>,
{
	WorkerBuilder::new().unwrap().spawn_async(f)
}
