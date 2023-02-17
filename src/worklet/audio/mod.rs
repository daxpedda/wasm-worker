mod module;

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use js_sys::{Array, Reflect};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};

pub use self::module::{AudioWorkletModule, AudioWorkletModuleFuture};
use super::{Data, WorkletInitError, WorkletModuleError};

pub trait AudioWorkletExt {
	fn init_wasm<F: 'static + FnOnce(AudioWorkletContext) + Send>(
		&self,
		f: F,
	) -> Result<AudioWorkletFuture, WorkletInitError>;

	fn init_wasm_with_module<F: 'static + FnOnce(AudioWorkletContext) + Send>(
		&self,
		module: &AudioWorkletModule,
		f: F,
	) -> Result<AudioWorkletFuture, WorkletInitError>;
}

impl AudioWorkletExt for BaseAudioContext {
	fn init_wasm<F: 'static + FnOnce(AudioWorkletContext) + Send>(
		&self,
		f: F,
	) -> Result<AudioWorkletFuture, WorkletInitError> {
		let init = Reflect::get(self, &"__wasm_worker_init".into()).unwrap_throw();

		if let Some(init) = init.as_bool() {
			assert!(init);

			return Err(WorkletInitError);
		}

		assert!(init.is_undefined());
		Reflect::set(self, &"__wasm_worker_init".into(), &true.into()).unwrap_throw();

		Ok(AudioWorkletFuture(Some(Inner::Module {
			context: self.clone(),
			f: Box::new(|| f(AudioWorkletContext)),
			future: AudioWorkletModule::default(),
		})))
	}

	fn init_wasm_with_module<F: 'static + FnOnce(AudioWorkletContext) + Send>(
		&self,
		module: &AudioWorkletModule,
		f: F,
	) -> Result<AudioWorkletFuture, WorkletInitError> {
		let init = Reflect::get(self, &"__wasm_worker_init".into()).unwrap_throw();

		if let Some(init) = init.as_bool() {
			assert!(init);

			return Err(WorkletInitError);
		}

		assert!(init.is_undefined());
		Reflect::set(self, &"__wasm_worker_init".into(), &true.into()).unwrap_throw();

		Ok(AudioWorkletFuture(Some(AudioWorkletFuture::new_add(
			self.clone(),
			Box::new(|| f(AudioWorkletContext)),
			module,
		))))
	}
}

#[derive(Clone, Copy, Debug)]
pub struct AudioWorkletContext;

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AudioWorkletFuture(Option<Inner>);

enum Inner {
	Module {
		context: BaseAudioContext,
		f: Box<dyn 'static + FnOnce() + Send>,
		future: AudioWorkletModuleFuture,
	},
	Add {
		context: BaseAudioContext,
		f: Box<dyn 'static + FnOnce() + Send>,
		future: JsFuture,
	},
}

impl Debug for Inner {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Module {
				context, future, ..
			} => formatter
				.debug_struct("Module")
				.field("context", context)
				.field("f", &"Box<FnOnce()>")
				.field("future", future)
				.finish(),
			Self::Add {
				context, future, ..
			} => formatter
				.debug_struct("Add")
				.field("context", context)
				.field("f", &"Box<FnOnce()>")
				.field("future", future)
				.finish(),
		}
	}
}

impl AudioWorkletFuture {
	fn new_add(
		context: BaseAudioContext,
		f: Box<dyn 'static + FnOnce() + Send>,
		module: &AudioWorkletModule,
	) -> Inner {
		let promise = context
			.audio_worklet()
			.unwrap_throw()
			.add_module(&module.0)
			.unwrap_throw();

		Inner::Add {
			context,
			f,
			future: JsFuture::from(promise),
		}
	}
}

impl Future for AudioWorkletFuture {
	type Output = Result<(), WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				Inner::Module { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(Inner::Module {context, f, ..}) = self.0.take() else {unreachable!()};

					let module = result?;

					self.0 = Some(Self::new_add(context, f, module));
				}
				Inner::Add { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(Inner::Add { context, f, ..}) = self.0.take() else {unreachable!()};

					assert!(result.unwrap_throw().is_undefined());

					let data = Box::into_raw(Box::new(Data(f)));

					let mut options = AudioWorkletNodeOptions::new();
					options.processor_options(Some(&Array::of3(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
					)));

					let result = AudioWorkletNode::new_with_options(
						&context,
						"__wasm_worker_InitWasm",
						&options,
					);
					assert!(result.unwrap_throw().is_undefined());

					return Poll::Ready(Ok(()));
				}
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for AudioWorkletFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}
