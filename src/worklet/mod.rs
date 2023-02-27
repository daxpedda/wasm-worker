mod builder;
mod context;
mod url;
mod worklet;

use web_sys::BaseAudioContext;

pub use self::builder::{WorkletBuilder, WorkletFuture, WorkletInitError};
pub use self::context::WorkletContext;
pub use self::url::{ImportSupportFuture, WorkletUrl, WorkletUrlError, WorkletUrlFuture};
pub use self::worklet::{Worklet, WorkletRef};

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

mod sealed {
	pub trait Sealed {}

	impl Sealed for web_sys::BaseAudioContext {}
}
