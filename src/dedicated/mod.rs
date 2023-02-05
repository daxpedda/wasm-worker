mod builder;
mod common;
mod context;
mod event;
mod handle;

use std::future::Future;

pub use self::builder::{ModuleSupportError, WorkerBuilder};
use self::common::WorkerOrContext;
pub use self::context::WorkerContext;
pub use self::event::{MessageEvent, MessageIter};
pub use self::handle::WorkerHandle;

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Close {
	Yes,
	No,
}

impl Close {
	const fn to_bool(self) -> bool {
		match self {
			Self::Yes => true,
			Self::No => false,
		}
	}
}
