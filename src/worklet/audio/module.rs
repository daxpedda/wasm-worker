use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use js_sys::Array;
use once_cell::sync::OnceCell;
use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{Blob, BlobPropertyBag, Url};

use crate::worklet::{DefaultWorkletModuleFuture, WorkletModule, WorkletModuleError};

static DEFAULT: OnceCell<AudioWorkletModule> = OnceCell::new();

#[derive(Debug)]
pub struct AudioWorkletModule(pub(super) String);

impl Drop for AudioWorkletModule {
	fn drop(&mut self) {
		Url::revoke_object_url(&self.0).unwrap_throw();
	}
}

impl AudioWorkletModule {
	#[allow(clippy::should_implement_trait)]
	pub fn default() -> AudioWorkletModuleFuture {
		AudioWorkletModuleFuture(Some(WorkletModule::default()))
	}

	#[must_use]
	pub fn new(WorkletModule { shim, imports }: &WorkletModule) -> Self {
		let sequence = if let Some(imports) = imports {
			Array::of3(
				&shim.into(),
				&imports.into(),
				&include_str!("worklet.js").into(),
			)
		} else {
			Array::of2(&shim.into(), &include_str!("worklet.js").into())
		};

		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property).unwrap_throw();
		let url = Url::create_object_url_with_blob(&blob).unwrap_throw();

		Self(url)
	}

	#[must_use]
	pub fn as_raw(&self) -> &str {
		&self.0
	}
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AudioWorkletModuleFuture(Option<DefaultWorkletModuleFuture>);

impl AudioWorkletModuleFuture {
	#[track_caller]
	pub fn into_inner(&mut self) -> Option<Result<&'static AudioWorkletModule, JsValue>> {
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
		let inner = self.0.as_mut().expect("polled after `Ready`");

		if let Some(default) = DEFAULT.get() {
			self.0.take();

			return Poll::Ready(Ok(default));
		}

		let result = ready!(Pin::new(inner).poll(cx));
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
