mod builder;
mod context;
mod url;
mod worklet;

use web_sys::BaseAudioContext;

pub use self::builder::{WorkletBuilder, WorkletFuture, WorkletInitError};
pub use self::context::WorkletContext;
pub use self::url::{ImportSupportFuture, WorkletUrl, WorkletUrlError, WorkletUrlFuture};
pub use self::worklet::{Worklet, WorkletRef};
use crate::common::WAIT_ASYNC_SUPPORT;

pub trait WorkletExt: sealed::Sealed {
	fn add_wasm<F>(&self, f: F) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send;
}

impl WorkletExt for BaseAudioContext {
	fn add_wasm<F>(&self, f: F) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		WorkletBuilder::new().add(self, f)
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
