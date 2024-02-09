//! Audio worklet extension implementations.

mod js;

use std::any;
use std::cell::RefCell;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use js_sys::{Array, Object};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, DomException,
};

use self::js::BaseAudioContextExt;
use super::js::META;
use super::memory::ThreadMemory;
use super::oneshot::Receiver;
use super::url::ScriptUrl;
use super::{oneshot, Thread, MAIN_THREAD};
use crate::web::audio_worklet::ExtendAudioWorkletProcessor;

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super) fn register_thread<F>(
	context: BaseAudioContext,
	task: F,
) -> RegisterThreadFuture
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
pub(in super::super) struct RegisterThreadFuture(Option<State>);

/// State of [`RegisterThreadFuture`].
enum State {
	/// Early error.
	Error(Error),
	/// Waiting for `Worklet.addModule()`.
	Module {
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
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
		context: BaseAudioContext,
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

impl Debug for State {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
			Self::Module {
				context,
				promise,
				task,
				receiver,
			} => formatter
				.debug_struct("Module")
				.field("context", context)
				.field("promise", promise)
				.field("task", &any::type_name_of_val(task))
				.field("receiver", receiver)
				.finish(),
			Self::Package { context, receiver } => formatter
				.debug_struct("Module")
				.field("context", context)
				.field("receiver", receiver)
				.finish(),
		}
	}
}

impl Drop for RegisterThreadFuture {
	fn drop(&mut self) {
		let Some(state) = self.0.take() else { return };

		if !matches!(state, State::Error(_)) {
			wasm_bindgen_futures::spawn_local(async move {
				let _ = Self(Some(state)).await;
			});
		}
	}
}

impl Future for RegisterThreadFuture {
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
						// Before spawning a new thread make sure we initialize [`MAIN_THREAD`].
						MAIN_THREAD.get_or_init(super::current_id);

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
					context,
					mut receiver,
				} => match Pin::new(&mut receiver).poll(cx) {
					Poll::Ready(Some(Package { thread, memory })) => {
						if let AudioContextState::Closed = context.state() {
							// SAFETY: When reaching the `closed` state, all resources
							// should have been freed. See <https://webaudio.github.io/web-audio-api/#dom-audiocontextstate-closed>.
							unsafe { memory.destroy() };
						} else {
							Self::schedule_clean(context, memory);
						}
						return Poll::Ready(Ok(thread));
					}
					Poll::Pending => {
						self.0 = Some(State::Package { context, receiver });
						return Poll::Pending;
					}
					Poll::Ready(None) => unreachable!("`Sender` dropped somehow"),
				},
			}
		}
	}
}

impl RegisterThreadFuture {
	/// Create a [`RegisterThreadFuture`] that returns `error`.
	pub(in super::super) const fn error(error: Error) -> Self {
		Self(Some(State::Error(error)))
	}

	/// Schedule thread cleanup.
	fn schedule_clean(context: BaseAudioContext, memory: ThreadMemory) {
		/// Hold data necessary to schedule the cleanup.
		struct Data {
			/// The corresponding [`BaseAudioContext`].
			context: BaseAudioContext,
			/// The corresponding [`ThreadMemory`].
			memory: ThreadMemory,
			/// The [`Closure`] to clean up.
			closure: Closure<dyn FnMut()>,
		}

		let data_rc = Rc::new(RefCell::new(None));

		let closure = Closure::new({
			let data_rc = Rc::clone(&data_rc);
			move || {
				let data: Data = data_rc
					.borrow_mut()
					.take()
					.expect("`BaseAudioContext` reached `closed` state twice");

				if let AudioContextState::Closed = data.context.state() {
					// SAFETY: When reaching the `closed` state, all resources
					// should have been freed. See <https://webaudio.github.io/web-audio-api/#dom-audiocontextstate-closed>.
					unsafe { data.memory.destroy() };

					// Remove the event listener.
					data.context
						.remove_event_listener_with_callback(
							"statechange",
							data.closure.as_ref().unchecked_ref(),
						)
						.expect("`EventTarget.removeEventListener()` is not expected to fail");
					// Don't drop the closure while it is being run.
					js::queue_microtask(
						&Closure::once_into_js(move || drop(data.closure)).unchecked_into(),
					);
				} else {
					*data_rc.borrow_mut() = Some(data);
				}
			}
		});
		context
			.add_event_listener_with_callback("statechange", closure.as_ref().unchecked_ref())
			.expect("`EventTarget.addEventListener()` is not expected to fail");
		*data_rc.borrow_mut() = Some(Data {
			context,
			memory,
			closure,
		});
	}
}

/// Determined if the current thread is the main thread.
pub(in super::super) fn is_main_thread() -> bool {
	*MAIN_THREAD.get_or_init(super::current_id) == super::current_id()
}

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
pub(in super::super) fn register_processor<T: 'static + ExtendAudioWorkletProcessor>(
	name: &str,
) -> Result<(), Error> {
	__web_thread_register_processor(
		name,
		__WebThreadProcessorConstructor(Box::new(ProcessorConstructorWrapper::<T>(PhantomData))),
	)
	.map_err(|error| error_from_exception(error.into()))
}

/// Holds the user-supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen]
struct __WebThreadProcessorConstructor(Box<dyn ProcessorConstructor>);

#[wasm_bindgen]
impl __WebThreadProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		self.0.instantiate(this, options)
	}

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	#[wasm_bindgen(js_name = parameterDescriptors)]
	#[allow(unreachable_pub)]
	pub fn parameter_descriptors(&self) -> Array {
		self.0.parameter_descriptors()
	}
}

/// Wrapper for the user-supplied [`ExtendAudioWorkletProcessor`].
struct ProcessorConstructorWrapper<T: 'static + ExtendAudioWorkletProcessor>(PhantomData<T>);

/// Object-safe version of [`ExtendAudioWorkletProcessor`].
trait ProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor;

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	fn parameter_descriptors(&self) -> Array;
}

impl<T: 'static + ExtendAudioWorkletProcessor> ProcessorConstructor
	for ProcessorConstructorWrapper<T>
{
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		__WebThreadProcessor(Box::new(T::new(this, options)))
	}

	fn parameter_descriptors(&self) -> Array {
		T::parameter_descriptors()
	}
}

/// Holds the user-supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen]
struct __WebThreadProcessor(Box<dyn ExtendAudioWorkletProcessor>);

#[wasm_bindgen]
impl __WebThreadProcessor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		self.0.process(inputs, outputs, parameters)
	}
}

/// Convert a [`JsValue`] to an [`DomException`] and then to an [`Error`].
fn error_from_exception(error: JsValue) -> Error {
	let error: DomException = error.unchecked_into();

	Error::other(format!("{}: {}", error.name(), error.message()))
}

/// Entry function for the worklet.
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __web_thread_worklet_entry(task: *mut Box<dyn FnOnce() + Send>) {
	// SAFETY: Has to be a valid pointer to a `Box<dyn FnOnce() + Send>`. We only
	// call `__web_thread_worker_entry` from `worklet.js`. The data sent to it
	// should only come from `RegisterThreadFuture::poll()`.
	let task = *unsafe { Box::from_raw(task) };
	task();
}

/// Entry function for the worklet.
#[wasm_bindgen]
#[allow(unreachable_pub)]
extern "C" {
	#[wasm_bindgen(catch)]
	fn __web_thread_register_processor(
		name: &str,
		processor: __WebThreadProcessorConstructor,
	) -> Result<(), DomException>;
}
