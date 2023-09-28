mod builder;
mod context;
mod handle;
mod url;

use std::borrow::Cow;
use std::future::Future;

use web_sys::BaseAudioContext;

pub use self::builder::{WorkletBuilder, WorkletFuture, WorkletInitError};
pub use self::context::WorkletContext;
pub use self::handle::Worklet;
#[cfg(feature = "message")]
pub use self::handle::WorkletRef;
use self::url::WORKLET_URL;
use crate::common::WAIT_ASYNC_SUPPORT;

pub trait WorkletExt: sealed::Sealed {
	#[track_caller]
	fn add_wasm<F>(&self, f: F) -> WorkletFuture<'_>
	where
		F: 'static + FnOnce(WorkletContext) + Send;

	#[track_caller]
	fn add_wasm_async<F1, F2>(&self, f: F1) -> WorkletFuture<'_>
	where
		F1: 'static + FnOnce(WorkletContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>;
}

impl WorkletExt for BaseAudioContext {
	fn add_wasm<F>(&self, task: F) -> WorkletFuture<'_>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		WorkletBuilder::new()
			.add(Cow::Borrowed(self), task)
			.unwrap()
	}

	fn add_wasm_async<F1, F2>(&self, task: F1) -> WorkletFuture<'_>
	where
		F1: 'static + FnOnce(WorkletContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		WorkletBuilder::new()
			.add_async(Cow::Borrowed(self), task)
			.unwrap()
	}
}

#[must_use]
pub fn has_async_support() -> bool {
	*WAIT_ASYNC_SUPPORT
}

mod sealed {
	pub trait Sealed {}

	impl Sealed for web_sys::BaseAudioContext {}
}
