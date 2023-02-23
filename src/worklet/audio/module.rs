use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use js_sys::Array;
use once_cell::sync::OnceCell;
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::worklet::module::Type;
use crate::worklet::{
	PolyfillImport, PolyfillInline, WorkletModule, WorkletModuleError, WorkletModuleFuture,
};

static DEFAULT: OnceCell<AudioWorkletModule> = OnceCell::new();

#[derive(Debug)]
pub struct AudioWorkletModule(pub(super) String);

impl Drop for AudioWorkletModule {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap();
	}
}

impl AudioWorkletModule {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> AudioWorkletModuleFuture {
		AudioWorkletModuleFuture(Some(WorkletModule::default()))
	}

	#[must_use]
	pub fn new(WorkletModule(r#type): &WorkletModule) -> Self {
		let sequence = match r#type {
			Type::Import(import) => Array::of3(
				&PolyfillImport::import().into(),
				&import.into(),
				&include_str!("worklet.js").into(),
			),
			Type::Inline { shim, imports } => Array::of4(
				&PolyfillInline::script().into(),
				&shim.into(),
				&imports.into(),
				&include_str!("worklet.js").into(),
			),
		};

		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap();
		let url = Url::create_object_url_with_blob(&blob).unwrap();

		Self(url)
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.0
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AudioWorkletModuleFuture(Option<WorkletModuleFuture<'static, 'static, true>>);

impl AudioWorkletModuleFuture {
	#[track_caller]
	pub fn into_inner(
		&mut self,
	) -> Option<Result<&'static AudioWorkletModule, WorkletModuleError>> {
		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Some(Ok(default));
		}

		if let Some(result) = self.0.as_mut().expect("polled after `Ready`").into_inner() {
			self.0.take();

			Some(match result {
				Ok(module) => Ok(DEFAULT.get_or_init(|| AudioWorkletModule::new(module))),
				Err(error) => Err(error),
			})
		} else {
			None
		}
	}
}

impl Future for AudioWorkletModuleFuture {
	type Output = Result<&'static AudioWorkletModule, WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let future = self.0.as_mut().expect("polled after `Ready`");

		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Poll::Ready(Ok(default));
		}

		let result = ready!(Pin::new(future).poll(cx));
		self.0.take();

		match result {
			Ok(module) => Poll::Ready(Ok(DEFAULT.get_or_init(|| AudioWorkletModule::new(module)))),
			Err(error) => Poll::Ready(Err(error)),
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for AudioWorkletModuleFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
