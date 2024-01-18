//! Implementation when the atomics target feature is enabled.

mod channel;
mod parker;
mod url;

use std::fmt::{self, Debug, Formatter};
use std::io;
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use atomic_waker::AtomicWaker;
use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use self::channel::Sender;
pub(super) use self::parker::Parker;
use self::url::URL;
use super::{JoinHandle, Thread, ThreadId};
use crate::thread;

/// Saves the [`ThreadId`] of the main thread.
static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();
/// [`Sender`] to the main thread.
static SENDER: OnceLock<Sender<Spawn>> = OnceLock::new();

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

/// Wrapper around user-spawned thread.
struct Spawn {
	/// Task.
	task: Box<dyn FnOnce() + Send>,
	/// Name of the thread.
	name: Option<String>,
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

	let task: Box<dyn FnOnce() + Send> = Box::new({
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
		}
	});

	if *MAIN_THREAD.get_or_init(|| thread::current().id()) == thread::current().id() {
		SENDER.get_or_init(|| {
			let (sender, receiver) = channel::channel::<Spawn>();

			wasm_bindgen_futures::spawn_local(async move {
				while let Ok(spawn) = receiver.next().await {
					spawn_internal(spawn.task, spawn.name.as_deref());
				}
			});

			sender
		});

		spawn_internal(task, name.as_deref());
	} else {
		SENDER
			.get()
			.expect("not spawning from main thread without initializing `SENDER`")
			.send(Spawn { task, name })
			.expect("`Receiver` was somehow dropped from the main thread");
	}

	Ok(JoinHandle { shared, thread })
}

/// Spawning thread regardless of being nested.
fn spawn_internal(task: Box<dyn FnOnce()>, name: Option<&str>) {
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
}

#[doc(hidden)]
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __web_thread_entry(data: *mut Box<dyn FnOnce()>) {
	js_sys::global()
		.unchecked_into::<DedicatedWorkerGlobalScope>()
		.set_onmessage(None);

	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__web_thread_entry` from `worker.js`. The data sent to it should
	// only come from `self::spawn_internal()`.
	let data = *unsafe { Box::from_raw(data) };
	data();
}
