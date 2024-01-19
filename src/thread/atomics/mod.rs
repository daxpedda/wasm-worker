//! Implementation when the atomics target feature is enabled.

mod channel;
mod js;
mod parker;
mod url;
mod wait_async;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use atomic_waker::AtomicWaker;
use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use self::channel::Sender;
use self::js::META;
pub(super) use self::parker::Parker;
use self::url::ScriptUrl;
use self::wait_async::Atomics;
use super::{JoinHandle, Thread, ThreadId};
use crate::thread;

/// Saves the [`ThreadId`] of the main thread.
static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();
/// [`Sender`] to the main thread.
static SENDER: OnceLock<Sender<Command>> = OnceLock::new();

thread_local! {
	/// Containing all spawned workers.
	static WORKERS: RefCell<HashMap<ThreadId, Worker>> = RefCell::new(HashMap::new());
}

/// Shared state between thread and [`JoinHandle`].
pub(crate) struct Shared<T> {
	/// [`Mutex`] holding the returned value.
	pub(crate) value: Mutex<Option<T>>,
	/// [`Condvar`] to wake up any thread waiting on the return value.
	pub(super) cvar: Condvar,
	/// Registered [`Waker`](std::task::Waker) to be notified when the thread is
	/// finished.
	pub(crate) waker: AtomicWaker,
}

impl<T> Debug for Shared<T> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Inner")
			.field("value", &"Mutex")
			.field("cvar", &self.cvar)
			.field("waker", &self.waker)
			.finish()
	}
}

/// Command sent to the main thread.
enum Command {
	/// Spawn a new thread.
	Spawn {
		/// [`ThreadId`] of the thread to be spawned.
		id: ThreadId,
		/// Task.
		task: Box<dyn FnOnce() -> u32 + Send>,
		/// Name of the thread.
		name: Option<String>,
	},
	/// Terminate thread.
	Terminate {
		/// [`ThreadId`] of the thread to be terminated.
		id: ThreadId,
		/// Value to use `Atomics.waitAsync` on.
		value: Box<i32>,
	},
}

/// Internal spawn function.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn spawn<F, T>(task: F, name: Option<String>) -> io::Result<JoinHandle<T>>
where
	F: 'static + FnOnce() -> T + Send,
	T: 'static + Send,
{
	let thread = Thread::new();
	let shared = Arc::new(Shared {
		value: Mutex::new(None),
		cvar: Condvar::new(),
		waker: AtomicWaker::new(),
	});

	let task: Box<dyn FnOnce() -> u32 + Send> = Box::new({
		let thread = thread.clone();
		let shared = Arc::downgrade(&shared);
		move || {
			Thread::register(thread);
			let value = task();

			if let Some(shared) = shared.upgrade() {
				*shared.value.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
				shared.cvar.notify_one();
				shared.waker.wake();
			}

			let value = Box::new(0);
			let index: *const i32 = value.deref();
			#[allow(clippy::as_conversions)]
			let index = index as u32 / 4;

			SENDER
				.get()
				.expect("closing thread without `SENDER` being initialized")
				.send(Command::Terminate {
					id: thread::current().id(),
					value,
				})
				.expect("`Receiver` was somehow dropped from the main thread");

			index
		}
	});

	if *MAIN_THREAD.get_or_init(|| thread::current().id()) == thread::current().id() {
		SENDER.get_or_init(|| {
			let (sender, receiver) = channel::channel::<Command>();

			wasm_bindgen_futures::spawn_local(async move {
				while let Ok(command) = receiver.next().await {
					match command {
						Command::Spawn { id, task, name } => {
							spawn_internal(id, task, name.as_deref());
						}
						Command::Terminate { id, value } => {
							wasm_bindgen_futures::spawn_local(async move {
								Atomics::wait_async(&value, 0).await;

								WORKERS
									.with(|workers| {
										workers
											.borrow_mut()
											.remove(&id)
											.expect("`Worker` to be destroyed not found")
									})
									.terminate();
							});
						}
					}
				}
			});

			sender
		});

		spawn_internal(thread.id(), task, name.as_deref());
	} else {
		SENDER
			.get()
			.expect("not spawning from main thread without initializing `SENDER`")
			.send(Command::Spawn {
				id: thread.id(),
				task,
				name,
			})
			.expect("`Receiver` was somehow dropped from the main thread");
	}

	Ok(JoinHandle { shared, thread })
}

/// Spawning thread regardless of being nested.
fn spawn_internal(id: ThreadId, task: Box<dyn FnOnce() -> u32>, name: Option<&str>) {
	thread_local! {
		/// Object URL to the worker script.
		static URL: ScriptUrl = ScriptUrl::new(&{
			format!(
				"import {{initSync, __web_thread_entry}} from '{}';\n\n{}",
				META.url(),
				include_str!("worker.js")
			)
		});
	}

	let mut options = WorkerOptions::new();
	options.type_(WorkerType::Module);

	if let Some(name) = name {
		options.name(name);
	}

	let worker = URL
		.with(|url| Worker::new_with_options(url.as_raw(), &options))
		.expect("`new Worker()` is not expected to fail with a local script");

	let message = Array::of3(
		&wasm_bindgen::module(),
		&wasm_bindgen::memory(),
		&Box::into_raw(Box::new(task)).into(),
	);

	worker
		.post_message(&message)
		.expect("`Worker.postMessage` is not expected to fail without a `transfer` object");

	assert_eq!(
		WORKERS.with(|workers| workers.borrow_mut().insert(id, worker)),
		None,
		"found previous worker with the same `ThreadId`"
	);
}

#[doc(hidden)]
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __web_thread_entry(task: *mut Box<dyn FnOnce() -> u32>) -> u32 {
	js_sys::global()
		.unchecked_into::<DedicatedWorkerGlobalScope>()
		.set_onmessage(None);

	// SAFETY: Has to be a valid pointer to a `Box<dyn FnOnce() -> i32>`. We only
	// call `__web_thread_entry` from `worker.js`. The data sent to it should
	// only come from `self::spawn_internal()`.
	let task = *unsafe { Box::from_raw(task) };
	task()
}