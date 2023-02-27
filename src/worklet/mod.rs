mod context;
mod future;
mod url;

use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter};

use js_sys::Reflect;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, BaseAudioContext};

pub use self::context::WorkletContext;
pub use self::future::WorkletFuture;
pub use self::url::{ImportSupportFuture, WorkletUrl, WorkletUrlError, WorkletUrlFuture};

pub trait WorkletExt: sealed::Sealed {
	fn add_wasm<F>(&self, f: F) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send;

	fn add_wasm_with_url<F>(
		&self,
		url: &WorkletUrl,
		f: F,
	) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send;
}

impl WorkletExt for BaseAudioContext {
	fn add_wasm<F>(&self, f: F) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		add_wasm_internal(self)?;

		Ok(WorkletFuture::new_url(
			Cow::Borrowed(self),
			Box::new(|global, id| {
				let context = WorkletContext::init(global, id);
				f(context);
			}),
			WorkletUrl::default(),
		))
	}

	fn add_wasm_with_url<F>(
		&self,
		url: &WorkletUrl,
		f: F,
	) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		add_wasm_internal(self)?;

		Ok(WorkletFuture::new_add(
			Cow::Borrowed(self),
			Box::new(|global, id| {
				let context = WorkletContext::init(global, id);
				f(context);
			}),
			url,
		))
	}
}

fn add_wasm_internal(this: &BaseAudioContext) -> Result<(), WorkletInitError> {
	let init = Reflect::get(this, &"__wasm_worker_init".into()).unwrap();

	if let Some(init) = init.as_bool() {
		debug_assert!(init);

		return Err(WorkletInitError);
	}

	debug_assert!(init.is_undefined());
	Reflect::set(this, &"__wasm_worker_init".into(), &true.into()).unwrap();

	Ok(())
}

mod sealed {
	pub trait Sealed {}

	impl Sealed for web_sys::BaseAudioContext {}
}

#[doc(hidden)]
#[allow(missing_debug_implementations, unreachable_pub)]
pub struct Data {
	id: usize,
	task: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, usize) + Send>,
}

#[doc(hidden)]
#[wasm_bindgen]
pub unsafe fn __wasm_worker_worklet_entry(data: *mut Data) {
	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worklet_entry` from `worklet.js`. The data sent to it should
	// only come from `WorkletExt::add_wasm_internal()`.
	let data = *unsafe { Box::from_raw(data) };

	let global = js_sys::global().unchecked_into();

	(data.task)(global, data.id);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WorkletInitError;

impl Display for WorkletInitError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "already initialized this worklet")
	}
}

impl Error for WorkletInitError {}
