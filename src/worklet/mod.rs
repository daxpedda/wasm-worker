mod builder;
mod context;
mod url;
mod worklet;

use std::future::Future;

use web_sys::BaseAudioContext;

pub use self::builder::{WorkletBuilder, WorkletFuture, WorkletInitError};
pub use self::context::WorkletContext;
pub use self::url::{WorkletUrl, WorkletUrlError, WorkletUrlFuture};
pub use self::worklet::Worklet;
#[cfg(feature = "message")]
pub use self::worklet::WorkletRef;
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
	fn add_wasm<F>(&self, f: F) -> WorkletFuture<'_>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		WorkletBuilder::new().add(self, f).unwrap()
	}

	fn add_wasm_async<F1, F2>(&self, f: F1) -> WorkletFuture<'_>
	where
		F1: 'static + FnOnce(WorkletContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		WorkletBuilder::new().add_async(self, f).unwrap()
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
