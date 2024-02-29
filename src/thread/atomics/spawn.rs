//! Thread spawning implementation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, OnceLock};
use std::{io, mem};

use js_sys::Array;
use js_sys::WebAssembly::{Memory, Module};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{Worker, WorkerOptions, WorkerType};

#[cfg(feature = "audio-worklet")]
use super::audio_worklet::register::THREAD_LOCK_INDEXES;
use super::channel::Sender;
use super::js::META;
use super::memory::ThreadMemory;
use super::url::ScriptUrl;
use super::wait_async::WaitAsync;
use super::{channel, oneshot, JoinHandle, ScopeData, Thread, ThreadId, MEMORY, MODULE};

/// [`Sender`] to the main thread.
static THREAD_HANDLER: OnceLock<Sender<Command>> = OnceLock::new();

thread_local! {
	/// Containing all spawned workers.
	static WORKERS: RefCell<HashMap<ThreadId, WorkerState>> = RefCell::new(HashMap::new());
}

/// State for each [`Worker`].
struct WorkerState {
	/// [`Worker`]
	this: Worker,
}

/// Type of the task being sent to the worker.
type Task<'scope> =
	Box<dyn 'scope + FnOnce() -> Pin<Box<dyn 'scope + Future<Output = u32>>> + Send>;

/// Command sent to the main thread.
enum Command {
	/// Spawn a new thread.
	Spawn(SpawnData<Task<'static>>),
	/// Terminate thread.
	Terminate {
		/// [`ThreadId`] of the thread to be terminated.
		id: ThreadId,
		/// Value to use `Atomics.waitAsync` on.
		value: Pin<Box<AtomicI32>>,
		/// Memory to destroy the thread.
		memory: ThreadMemory,
	},
}

/// Data to spawn new thread.
struct SpawnData<T> {
	/// [`ThreadId`] of the thread to be spawned.
	id: ThreadId,
	/// Task.
	task: T,
	/// Name of the thread.
	name: Option<String>,
}

/// Initializes the main thread thread handler. Make sure to call this at
/// least once on the main thread before spawning any thread.
///
/// # Panics
///
/// This will panic if called outside the main thread.
pub(super) fn init_main_thread() {
	debug_assert!(
		super::is_main_thread(),
		"initizalizing main thread without being on the main thread"
	);

	THREAD_HANDLER.get_or_init(|| {
		super::has_spawn_support();

		let (sender, receiver) = channel::channel::<Command>();

		wasm_bindgen_futures::spawn_local(async move {
			while let Ok(command) = receiver.next().await {
				match command {
					Command::Spawn(SpawnData { id, task, name }) => {
						spawn_internal(id, task, name.as_deref());
					}
					Command::Terminate { id, value, memory } => {
						wasm_bindgen_futures::spawn_local(async move {
							WaitAsync::wait(&value, 0).await;

							// SAFETY: We wait until the execution block has exited and block the
							// thread afterwards.
							unsafe { memory.destroy() };

							WORKERS
								.with(|workers| {
									workers
										.borrow_mut()
										.remove(&id)
										.expect("`Worker` to be destroyed not found")
								})
								.this
								.terminate();
						});
					}
				}
			}
		});

		sender
	});
}

/// Internal spawn function.
///
/// # Safety
///
/// `task` has to outlive the thread.
#[allow(clippy::unnecessary_wraps)]
pub(super) unsafe fn spawn<F1, F2, T>(
	task: F1,
	name: Option<String>,
	scope: Option<Arc<ScopeData>>,
) -> io::Result<JoinHandle<T>>
where
	F1: FnOnce() -> F2 + Send,
	F2: Future<Output = T>,
	T: Send,
{
	let thread = thread_init(name, scope.as_deref());
	let (sender, receiver) = oneshot::channel();

	let task: Task<'_> = Box::new({
		let thread = thread.clone();
		move || thread_runner(thread, sender, scope, task)
	});

	if super::is_main_thread() {
		init_main_thread();

		spawn_internal(thread.id(), task, thread.name());
	} else {
		// SAFETY: `task` has to be `'static` or `scope` has to be `Some`, which
		// prevents this thread from outliving its lifetime.
		let task = unsafe { mem::transmute::<Task<'_>, Task<'static>>(task) };

		THREAD_HANDLER
			.get()
			.expect("not spawning from main thread without initializing `SENDER`")
			.send(Command::Spawn(SpawnData {
				id: thread.id(),
				task,
				name: thread.0.name.clone(),
			}))
			.expect("`Receiver` was somehow dropped from the main thread");
	}

	Ok(JoinHandle {
		receiver: Some(receiver),
		thread,
	})
}

/// Common functionality between thread spawning initialization, regardless if a
/// message is passed or not.
fn thread_init(name: Option<String>, scope: Option<&ScopeData>) -> Thread {
	let thread = Thread::new_with_name(name);

	if let Some(scope) = &scope {
		// This can't overflow because creating a `ThreadId` would fail beforehand.
		scope.threads.fetch_add(1, Ordering::Relaxed);
	}

	thread
}

/// Common functionality between threads, regardless if a message is passed.
fn thread_runner<'scope, T: 'scope + Send, F1: 'scope + FnOnce() -> F2, F2: Future<Output = T>>(
	thread: Thread,
	sender: oneshot::Sender<T>,
	scope: Option<Arc<ScopeData>>,
	task: F1,
) -> Pin<Box<dyn 'scope + Future<Output = u32>>> {
	Box::pin(async move {
		Thread::register(thread);
		sender.send(task().await);

		if let Some(scope) = scope {
			if scope.threads.fetch_sub(1, Ordering::Release) == 1 {
				scope.thread.unpark();
				scope.waker.wake();
			}
		}

		let value = Box::pin(AtomicI32::new(0));
		let index = super::i32_to_buffer_index(value.as_ptr());

		THREAD_HANDLER
			.get()
			.expect("closing thread without `SENDER` being initialized")
			.send(Command::Terminate {
				id: super::current_id(),
				value,
				memory: ThreadMemory::new(),
			})
			.expect("`Receiver` was somehow dropped from the main thread");

		index
	})
}

/// Spawning thread regardless of being nested.
fn spawn_internal<T>(id: ThreadId, task: T, name: Option<&str>) {
	spawn_common(id, task, name, |worker, module, memory, task| {
		#[cfg(not(feature = "audio-worklet"))]
		let message = Array::of3(module, memory, &task);
		#[cfg(feature = "audio-worklet")]
		let message = { THREAD_LOCK_INDEXES.with(|indexes| Array::of4(module, memory, indexes, &task)) };
		worker.post_message(&message)
	})
	.expect("`Worker.postMessage` is not expected to fail without a `transfer` object");
}

/// [`spawn_internal`] regardless if a message is passed or not.
fn spawn_common<T, E>(
	id: ThreadId,
	task: T,
	name: Option<&str>,
	post: impl FnOnce(&Worker, &Module, &Memory, JsValue) -> Result<(), E>,
) -> Result<(), E> {
	thread_local! {
		/// Object URL to the worker script.
		static URL: ScriptUrl = ScriptUrl::new(&{
			#[cfg(not(feature = "audio-worklet"))]
			let script = include_str!("worker.js");
			#[cfg(feature = "audio-worklet")]
			let script = include_str!("worker_with_audio_worklet.js");

			format!(
				"import {{initSync, __web_thread_worker_entry}} from '{}'\n\n{}",
				META.url(),
				script,
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

	let task = Box::into_raw(Box::new(task));

	if let Err(err) =
		MODULE.with(|module| MEMORY.with(|memory| post(&worker, module, memory, task.into())))
	{
		// SAFETY: We just made this pointer above and `post` has to guarantee that on
		// error transmission failed to avoid double-free.
		drop(unsafe { Box::from_raw(task) });
		return Err(err);
	};

	let previous = WORKERS.with(|workers| {
		workers
			.borrow_mut()
			.insert(id, WorkerState { this: worker })
	});
	debug_assert!(
		previous.is_none(),
		"found previous worker with the same `ThreadId`"
	);

	Ok(())
}

/// TODO: Remove when `wasm-bindgen` supports `'static` in functions.
type TaskStatic = Task<'static>;

/// Entry function for the worker.
///
/// # Safety
///
/// `task` has to be a valid pointer to [`Task`].
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub async unsafe fn __web_thread_worker_entry(task: *mut TaskStatic) -> u32 {
	// SAFETY: Has to be a valid pointer to a `Task`. We only call
	// `__web_thread_worker_entry` from `worker.js`. The data sent to it comes only
	// from `spawn_internal()`.
	let task = *unsafe { Box::from_raw(task) };
	task().await
}
