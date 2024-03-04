//! Registering an audio worklet thread on a [`BaseAudioContext`].

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Context, Poll};
use std::{any, io};

use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioContextState, AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};

use super::super::js::META;
use super::super::memory::ThreadMemory;
use super::super::oneshot::{self, Receiver};
use super::super::url::ScriptUrl;
use super::super::wait_async::WaitAsync;
use super::super::{main, Thread};
use super::js::BaseAudioContextExt;
use crate::thread::atomics::is_main_thread;

/// Type of the task being sent to the worklet.
type Task<'scope> = Box<dyn 'scope + FnOnce() + Send>;

/// Locks instantiating workers until worklets have finished instantiating.
static WORKLET_LOCK: AtomicI32 = AtomicI32::new(0);
/// Counts how many workers are currently instantiating.
static WORKER_LOCK: AtomicI32 = AtomicI32::new(0);

thread_local! {
	/// Cached [`JsValue`] holding index to worklet lock.
	pub(in super::super) static WORKLET_LOCK_INDEX: JsValue =
		super::super::i32_to_buffer_index(WORKLET_LOCK.as_ptr()).into();
	/// Cached [`Array`] holding indexes to worker and worklet locks.
	pub(in super::super) static THREAD_LOCK_INDEXES: Array =
		WORKLET_LOCK_INDEX.with(|worklet_index| {
			Array::of2(
				worklet_index,
				&super::super::i32_to_buffer_index(WORKER_LOCK.as_ptr()).into(),
			)
		});
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super::super) fn register_thread<F>(
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
				"import {{initSync, __web_thread_worklet_entry}} from '{}'\n\n{}",
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
					let thread = super::super::super::current();
					let memory = ThreadMemory::new();
					sender.send(Package { thread, memory });
					task();
				});

				State::Module {
					context,
					promise,
					task,
					receiver,
				}
			}
			Err(error) => State::Error(super::super::error_from_exception(error)),
		},
	))
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super::super) struct RegisterThreadFuture(Option<State>);

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
		/// Caller-supplied task.
		task: Task<'static>,
		/// Receiver for the [`Package`].
		receiver: Receiver<Package>,
	},
	/// Waiting for the worklet lock to be available.
	WorkletLock {
		/// [`Future`] waiting for the worklet lock to be available.
		future: Option<WaitAsync>,
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// Receiver for the [`Package`].
		receiver: Receiver<Package>,
		/// [`AudioWorkletNodeOptions`] to be passed into [`AudioWorkletNode`].
		options: AudioWorkletNodeOptions,
		/// Pointer to [`Task`]. Used to clean up task on failure.
		task: *mut Box<dyn FnOnce() + Send>,
	},
	/// Waiting for the worker lock to be available.
	WorkerLock {
		/// [`Future`] waiting for the worker lock to be available.
		future: Option<WaitAsync>,
		/// Corresponding [`BaseAudioContext`].
		context: BaseAudioContext,
		/// Receiver for the [`Package`].
		receiver: Receiver<Package>,
		/// [`AudioWorkletNodeOptions`] to be passed into [`AudioWorkletNode`].
		options: AudioWorkletNodeOptions,
		/// Pointer to [`Task`]. Used to clean up task on failure.
		task: *mut Box<dyn FnOnce() + Send>,
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
	/// Handle to release thread memory.
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
			Self::WorkletLock {
				future,
				context,
				receiver,
				options,
				task,
			}
			| Self::WorkerLock {
				future,
				context,
				receiver,
				options,
				task,
			} => formatter
				.debug_struct(match self {
					Self::WorkletLock { .. } => "WorkletLock",
					Self::WorkerLock { .. } => "WorkerLock",
					_ => unreachable!(),
				})
				.field("future", future)
				.field("context", context)
				.field("receiver", receiver)
				.field("options", options)
				.field("task", task)
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
	type Output = io::Result<AudioWorkletHandle>;

	#[allow(clippy::too_many_lines)]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		loop {
			let mut state = self.0.take().expect("polled after completion");

			match state {
				State::Error(error) => return Poll::Ready(Err(error)),
				State::Module {
					ref mut promise, ..
				} => match Pin::new(promise).poll(cx) {
					Poll::Ready(Ok(_)) => {
						// This is checked earlier.
						debug_assert!(
							is_main_thread(),
							"started registering thread without being on the main thread"
						);
						// Before spawning a new thread make sure we initialize the main thread.
						main::init_main_thread();

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
						options.processor_options(Some(&WORKLET_LOCK_INDEX.with(|index| {
							Array::of4(
								&wasm_bindgen::module(),
								&wasm_bindgen::memory(),
								index,
								&task.into(),
							)
						})));

						self.0 = Some(State::WorkletLock {
							future: None,
							context,
							receiver,
							options,
							task,
						});
					}
					Poll::Ready(Err(error)) => {
						return Poll::Ready(Err(super::super::error_from_exception(error)))
					}
					Poll::Pending => {
						self.0 = Some(state);
						return Poll::Pending;
					}
				},
				State::WorkletLock {
					future,
					context,
					receiver,
					options,
					task,
				} => {
					if let Some(mut future) = future {
						if Pin::new(&mut future).poll(cx).is_pending() {
							self.0 = Some(State::WorkletLock {
								future: Some(future),
								context,
								receiver,
								options,
								task,
							});
							return Poll::Pending;
						}
					}

					if WORKLET_LOCK
						.compare_exchange_weak(0, 1, Ordering::Relaxed, Ordering::Relaxed)
						.is_err()
					{
						self.0 = Some(State::WorkletLock {
							future: Some(WaitAsync::wait(&WORKLET_LOCK, 1)),
							context,
							receiver,
							options,
							task,
						});
						continue;
					}

					self.0 = Some(State::WorkerLock {
						future: None,
						context,
						receiver,
						options,
						task,
					});
				}
				State::WorkerLock {
					future,
					context,
					receiver,
					options,
					task,
				} => {
					if let Some(mut future) = future {
						if Pin::new(&mut future).poll(cx).is_pending() {
							self.0 = Some(State::WorkerLock {
								future: Some(future),
								context,
								receiver,
								options,
								task,
							});
							return Poll::Pending;
						}
					}

					let worker_lock = WORKER_LOCK.load(Ordering::Relaxed);

					if worker_lock != 0 {
						self.0 = Some(State::WorkerLock {
							future: Some(WaitAsync::wait(&WORKER_LOCK, worker_lock)),
							context,
							receiver,
							options,
							task,
						});
						continue;
					}

					match AudioWorkletNode::new_with_options(
						&context,
						"__web_thread_worklet",
						&options,
					) {
						Ok(_) => self.0 = Some(State::Package { context, receiver }),
						Err(error) => {
							// SAFETY: We just made this pointer above and `new
							// AudioWorkletNode` has to guarantee that on error transmission
							// failed to avoid double-free.
							drop(unsafe { Box::from_raw(task) });
							return Poll::Ready(Err(super::super::error_from_exception(error)));
						}
					}
				}
				State::Package {
					context,
					mut receiver,
				} => match Pin::new(&mut receiver).poll(cx) {
					Poll::Ready(Some(Package { thread, memory })) => {
						return Poll::Ready(Ok(AudioWorkletHandle { thread, memory }));
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
	pub(in super::super::super) const fn error(error: Error) -> Self {
		Self(Some(State::Error(error)))
	}
}

/// Implementation for [`crate::web::audio_worklet::AudioWorkletHandle`].
#[derive(Debug)]
pub(in super::super::super) struct AudioWorkletHandle {
	/// Corresponding [`Thread`].
	thread: Thread,
	/// Memory handle of the corresponding audio worklet thread.
	memory: ThreadMemory,
}

impl AudioWorkletHandle {
	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::thread()`].
	pub(crate) const fn thread(&self) -> &Thread {
		&self.thread
	}

	/// Implementation for
	/// [`crate::web::audio_worklet::AudioWorkletHandle::release()`].
	///
	/// # Safety
	///
	/// See [`ThreadMemory::release()`].
	pub(crate) unsafe fn release(self) -> Result<(), Self> {
		// SAFETY: See `ThreadMemory::release()`. Other safety guarantees have to be
		// uphold by the caller.
		unsafe { self.memory.release() }.map_err(|memory| Self {
			thread: self.thread,
			memory,
		})
	}
}

/// TODO: Remove when `wasm-bindgen` supports `'static` in functions.
type TaskStatic = Task<'static>;

/// Entry function for the worklet.
///
/// # Safety
///
/// `task` has to be a valid pointer to [`Task`].
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __web_thread_worklet_entry(task: *mut TaskStatic) {
	// SAFETY: Has to be a valid pointer to a `TaskStatic`. We only call
	// `__web_thread_worklet_entry` from `worklet.js`. The data sent to it comes
	// only from `RegisterThreadFuture::poll()`.
	let task = *unsafe { Box::from_raw(task) };
	task();
}
