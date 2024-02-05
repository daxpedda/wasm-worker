//! Audio worklet extension implementations.

mod js;

use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use js_sys::Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, DomException,
	Event,
};

use self::js::BaseAudioContextExt;
use super::js::META;
use super::memory::ThreadMemory;
use super::oneshot::Receiver;
use super::url::ScriptUrl;
use super::{oneshot, Thread};

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super) fn register_thread<F>(
	context: Cow<'_, BaseAudioContext>,
	task: F,
) -> RegisterThreadFuture<'_>
where
	F: 'static + FnOnce() + Send,
{
	thread_local! {
		/// Object URL to the worklet script.
		static URL: ScriptUrl = ScriptUrl::new(&{
			format!(
				"import {{initSync, __web_thread_worklet_entry}} from '{}';\n\n{}",
				META.url(),
				include_str!("worklet.js")
			)
		});
	}

	if let AudioContextState::Closed = context.state() {
		return RegisterThreadFuture(Some(State::Error(Error::other(
			"`BaseAudioContext` is closed",
		))));
	}

	if let Some(true) = context.unchecked_ref::<BaseAudioContextExt>().registered() {
		return RegisterThreadFuture(Some(State::Error(Error::new(
			ErrorKind::AlreadyExists,
			"`BaseAudioContext` already registered a thread",
		))));
	}

	let worklet = context
		.audio_worklet()
		.expect("`BaseAudioContext.audioWorklet` expected to be valid");

	RegisterThreadFuture(Some(
		match URL.with(|url| worklet.add_module(url.as_raw())) {
			Ok(promise) => {
				context
					.unchecked_ref::<BaseAudioContextExt>()
					.set_registered(true);
				let promise = JsFuture::from(promise);
				let (sender, receiver) = oneshot::channel();

				let task = Box::new(move || {
					let thread = super::super::current();
					let memory = ThreadMemory::new();
					sender.send(Package { thread, memory });
					task();
				});

				State::Module {
					context,
					promise,
					task: Box::new(task),
					receiver,
				}
			}
			Err(error) => State::Error(error_from_exception(error)),
		},
	))
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super) struct RegisterThreadFuture<'context>(Option<State<'context>>);

/// State of [`RegisterThreadFuture`].
enum State<'context> {
	/// Early error.
	Error(Error),
	/// Waiting for `Worklet.addModule()`.
	Module {
		/// Corresponding [`BaseAudioContext`].
		context: Cow<'context, BaseAudioContext>,
		/// `Promise` returned by `Worklet.addModule()`.
		promise: JsFuture,
		/// User-supplied task.
		task: Box<dyn 'static + FnOnce() + Send>,
		/// Receiver for the [`Package`].
		receiver: Receiver<Package>,
	},
	/// Waiting for [`Package`].
	Package {
		/// Corresponding [`BaseAudioContext`].
		context: Cow<'context, BaseAudioContext>,
		/// Receiver for the [`Package`].
		receiver: Receiver<Package>,
	},
}

/// Data sent by the spawned thread.
struct Package {
	/// [`Thread`].
	thread: Thread,
	/// Threads memory to destroy when we are done.
	memory: ThreadMemory,
}

impl Debug for State<'_> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
			Self::Module {
				context,
				promise,
				receiver,
				..
			} => formatter
				.debug_struct("Module")
				.field("context", context)
				.field("promise", promise)
				.field("receiver", receiver)
				.finish_non_exhaustive(),
			Self::Package { context, receiver } => formatter
				.debug_struct("Module")
				.field("context", context)
				.field("receiver", receiver)
				.finish(),
		}
	}
}

impl Drop for RegisterThreadFuture<'_> {
	fn drop(&mut self) {
		let Some(state) = self.0.take() else { return };

		if !matches!(state, State::Error(_)) {
			let future = Self(Some(state)).into_static();

			wasm_bindgen_futures::spawn_local(async move {
				let _ = future.await;
			});
		}
	}
}

impl Future for RegisterThreadFuture<'_> {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			let mut state = self.0.take().expect("polled after completion");

			match state {
				State::Error(error) => return Poll::Ready(Err(error)),
				State::Module {
					ref mut promise, ..
				} => match Pin::new(promise).poll(cx) {
					Poll::Ready(Ok(_)) => {
						let State::Module {
							context,
							task,
							receiver,
							..
						} = state
						else {
							unreachable!("found wrong state")
						};

						let task = Box::into_raw(Box::new(task));
						let mut options = AudioWorkletNodeOptions::new();
						options.processor_options(Some(&Array::of3(
							&wasm_bindgen::module(),
							&wasm_bindgen::memory(),
							&task.into(),
						)));

						match AudioWorkletNode::new_with_options(
							&context,
							"__web_thread_worklet",
							&options,
						) {
							Ok(_) => self.0 = Some(State::Package { context, receiver }),
							Err(error) => {
								// SAFETY: We have to assume that if this fails it never arrived at
								// the thread.
								drop(unsafe { Box::from_raw(task) });
								return Poll::Ready(Err(error_from_exception(error)));
							}
						}
					}
					Poll::Ready(Err(error)) => {
						return Poll::Ready(Err(error_from_exception(error)))
					}
					Poll::Pending => {
						self.0 = Some(state);
						return Poll::Pending;
					}
				},
				State::Package {
					#[allow(clippy::ref_patterns)]
					ref context,
					ref mut receiver,
				} => match Pin::new(receiver).poll(cx) {
					Poll::Ready(Some(Package { thread, memory })) => {
						let mut memory = Some(memory);
						let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
							let context: BaseAudioContext = event
								.target()
								.expect("`Event.target` is not expected to be empty")
								.unchecked_into();

							if let AudioContextState::Closed = context.state() {
								let memory = memory
									.take()
									.expect("`BaseAudioContext` reached `closed` state twice");
								// SAFETY: When reaching the `closed` state, all resources should
								// have been freed. See <https://webaudio.github.io/web-audio-api/#dom-audiocontextstate-closed>.
								unsafe { memory.destroy() }
							}
						});
						context
							.add_event_listener_with_callback(
								"statechange",
								closure.as_ref().unchecked_ref(),
							)
							.expect("`EventTarget.addEventListener()` is not expected to fail");
						closure.into_js_value();

						return Poll::Ready(Ok(thread));
					}
					Poll::Pending => {
						self.0 = Some(state);
						return Poll::Pending;
					}
					Poll::Ready(None) => unreachable!("`Sender` dropped somehow"),
				},
			}
		}
	}
}

impl RegisterThreadFuture<'_> {
	/// Create a [`RegisterThreadFuture`] that returns `error`.
	pub(in super::super) const fn error(error: Error) -> Self {
		Self(Some(State::Error(error)))
	}

	/// Remove the lifetime.
	pub(in super::super) fn into_static(mut self) -> RegisterThreadFuture<'static> {
		RegisterThreadFuture(Some(match self.0.take() {
			Some(State::Error(error)) => State::Error(error),
			Some(State::Module {
				context,
				promise,
				task,
				receiver,
			}) => State::Module {
				context: match context {
					Cow::Borrowed(context) => Cow::Owned(context.clone()),
					Cow::Owned(context) => Cow::Owned(context),
				},
				promise,
				task,
				receiver,
			},
			Some(State::Package { context, receiver }) => State::Package {
				context: match context {
					Cow::Borrowed(context) => Cow::Owned(context.clone()),
					Cow::Owned(context) => Cow::Owned(context),
				},
				receiver,
			},
			None => return RegisterThreadFuture(None),
		}))
	}
}

/// Convert a [`JsValue`] to an [`DomException`] and then to an [`Error`].
fn error_from_exception(error: JsValue) -> Error {
	let error: DomException = error.unchecked_into();

	Error::other(format!("{}: {}", error.name(), error.message()))
}
