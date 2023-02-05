mod builder;
mod common;
mod context;
mod event;
mod handle;
mod worker_url;

use std::future::Future;

pub use self::builder::{Close, ModuleSupportError, WorkerBuilder};
use self::common::WorkerOrContext;
pub use self::context::WorkerContext;
pub use self::event::{MessageEvent, MessageIter};
pub use self::handle::WorkerHandle;
pub use self::worker_url::{default_worker_url, WorkerUrl, WorkerUrlFormat};

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
}
