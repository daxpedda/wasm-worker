mod context;
mod url;

use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::future::FusedFuture;
use js_sys::{Array, Reflect};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, WorkletGlobalScope};

pub use self::context::AudioWorkletContext;
pub use self::url::{AudioWorkletUrl, AudioWorkletUrlFuture};
use super::{Data, WorkletInitError, WorkletModuleError};
use crate::common::ID_COUNTER;

pub trait AudioWorkletExt: sealed::Sealed {
	fn init_wasm<F>(&self, f: F) -> Result<AudioWorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(AudioWorkletContext) + Send;

	fn init_wasm_with_url<F>(
		&self,
		url: &AudioWorkletUrl,
		f: F,
	) -> Result<AudioWorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(AudioWorkletContext) + Send;
}

impl AudioWorkletExt for BaseAudioContext {
	fn init_wasm<F>(&self, f: F) -> Result<AudioWorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(AudioWorkletContext) + Send,
	{
		init_wasm_internal(self)?;

		Ok(AudioWorkletFuture(Some(State::Url {
			context: Cow::Borrowed(self),
			f: Box::new(|global, id| {
				let context = AudioWorkletContext::init(global, id);
				f(context);
			}),
			future: AudioWorkletUrl::default(),
		})))
	}

	fn init_wasm_with_url<F>(
		&self,
		url: &AudioWorkletUrl,
		f: F,
	) -> Result<AudioWorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(AudioWorkletContext) + Send,
	{
		init_wasm_internal(self)?;

		Ok(AudioWorkletFuture(Some(AudioWorkletFuture::new_add(
			Cow::Borrowed(self),
			Box::new(|global, id| {
				let context = AudioWorkletContext::init(global, id);
				f(context);
			}),
			url,
		))))
	}
}

fn init_wasm_internal(this: &BaseAudioContext) -> Result<(), WorkletInitError> {
	let init = Reflect::get(this, &"__wasm_worker_init".into()).unwrap();

	if let Some(init) = init.as_bool() {
		debug_assert!(init);

		return Err(WorkletInitError);
	}

	debug_assert!(init.is_undefined());
	Reflect::set(this, &"__wasm_worker_init".into(), &true.into()).unwrap();

	Ok(())
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AudioWorkletFuture<'context>(Option<State<'context>>);

enum State<'context> {
	Url {
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(WorkletGlobalScope, usize) + Send>,
		future: AudioWorkletUrlFuture,
	},
	Add {
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(WorkletGlobalScope, usize) + Send>,
		future: JsFuture,
	},
}

impl AudioWorkletFuture<'_> {
	pub fn into_static(self) -> AudioWorkletFuture<'static> {
		AudioWorkletFuture(match self.0 {
			Some(State::Url { context, f, future }) => Some(State::Url {
				context: Cow::Owned(context.into_owned()),
				f,
				future,
			}),
			Some(State::Add { context, f, future }) => Some(State::Add {
				context: Cow::Owned(context.into_owned()),
				f,
				future,
			}),
			None => None,
		})
	}

	fn new_add<'context>(
		context: Cow<'context, BaseAudioContext>,
		f: Box<dyn 'static + FnOnce(WorkletGlobalScope, usize) + Send>,
		url: &AudioWorkletUrl,
	) -> State<'context> {
		let promise = context.audio_worklet().unwrap().add_module(&url.0).unwrap();

		State::Add {
			context,
			f,
			future: JsFuture::from(promise),
		}
	}
}

impl Future for AudioWorkletFuture<'_> {
	type Output = Result<(), WorkletModuleError>;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			match self.0.as_mut().expect("polled after `Ready`") {
				State::Url { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Url {context, f, ..}) = self.0.take() else { unreachable!() };

					let url = result?;

					self.0 = Some(Self::new_add(context, f, url));
				}
				State::Add { future, .. } => {
					let result = ready!(Pin::new(future).poll(cx));
					let Some(State::Add { context, f, ..}) = self.0.take() else { unreachable!() };

					let result = result.unwrap();
					debug_assert!(result.is_undefined());

					let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
					let data = Box::into_raw(Box::new(Data { id, task: f }));

					let mut options = AudioWorkletNodeOptions::new();
					options.processor_options(Some(&Array::of3(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
					)));

					let _node = AudioWorkletNode::new_with_options(
						&context,
						"__wasm_worker_InitWasm",
						&options,
					)
					.unwrap();

					return Poll::Ready(Ok(()));
				}
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for AudioWorkletFuture<'_> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl Debug for State<'_> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Url {
				context, future, ..
			} => formatter
				.debug_struct("Url")
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

mod sealed {
	pub trait Sealed {}

	impl Sealed for web_sys::BaseAudioContext {}
}
