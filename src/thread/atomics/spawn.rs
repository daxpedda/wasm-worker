//! Thread spawning implementation.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::{io, mem};

use js_sys::Array;
use js_sys::WebAssembly::{Memory, Module};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{Worker, WorkerOptions, WorkerType};

#[cfg(feature = "audio-worklet")]
use super::audio_worklet::register::THREAD_LOCK_INDEXES;
use super::js::META;
use super::main::{self, Command};
use super::memory::ThreadMemory;
use super::oneshot::{self, Sender};
use super::url::ScriptUrl;
use super::{JoinHandle, ScopeData, Thread, ThreadId, MEMORY, MODULE};
use crate::thread::atomics::main::{WorkerState, WORKERS};

/// Type of the task being sent to the worker.
pub(super) type Task<'scope> =
	Box<dyn 'scope + FnOnce() -> Pin<Box<dyn 'scope + Future<Output = u32>>> + Send>;

/// Data to spawn new thread.
pub(super) struct SpawnData<T> {
	/// [`ThreadId`] of the thread to be spawned.
	pub(super) id: ThreadId,
	/// Task.
	pub(super) task: T,
	/// Name of the thread.
	pub(super) name: Option<String>,
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
		main::init_main_thread();

		spawn_internal(thread.id(), task, thread.name());
	} else {
		// SAFETY: `task` has to be `'static` or `scope` has to be `Some`, which
		// prevents this thread from outliving its lifetime.
		let task = unsafe { mem::transmute::<Task<'_>, Task<'static>>(task) };

		Command::Spawn(SpawnData {
			id: thread.id(),
			task,
			name: thread.0.name.clone(),
		})
		.send();
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
	sender: Sender<T>,
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

		Command::Terminate {
			id: super::current_id(),
			value,
			memory: ThreadMemory::new(),
		}
		.send();

		index
	})
}

/// Spawning thread regardless of being nested.
pub(super) fn spawn_internal<T>(id: ThreadId, task: T, name: Option<&str>) {
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
