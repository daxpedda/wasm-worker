//! Thread spawning implementation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::{fmt, io, mem};

use atomic_waker::AtomicWaker;
use js_sys::WebAssembly::Global;
use js_sys::{Array, Number, Object};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{Worker, WorkerOptions, WorkerType};

use super::channel::Sender;
use super::js::{Exports, GlobalDescriptor, META};
use super::url::ScriptUrl;
use super::wait_async::WaitAsync;
use super::{channel, JoinHandle, ScopeData, MEMORY, MODULE};
use crate::thread::{Thread, ThreadId, THREAD};

/// Saves the [`ThreadId`] of the main thread.
static MAIN_THREAD: OnceLock<ThreadId> = OnceLock::new();
/// [`Sender`] to the main thread.
static SENDER: OnceLock<Sender<Command>> = OnceLock::new();

thread_local! {
	/// Containing all spawned workers.
	static WORKERS: RefCell<HashMap<ThreadId, Worker>> = RefCell::new(HashMap::new());
}

/// Type of the task being sent to the worker.
type Task = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = u32>>> + Send>;
/// Type of the task being sent to the worker with `'static` lifetime.
type TaskStatic =
	Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = u32>>> + Send>;

/// Shared state between thread and [`JoinHandle`].
pub(super) struct Shared<T> {
	/// [`Mutex`] holding the returned value.
	pub(super) value: Mutex<Option<T>>,
	/// [`Condvar`] to wake up any thread waiting on the return value.
	pub(super) cvar: Condvar,
	/// Registered [`Waker`](std::task::Waker) to be notified when the thread is
	/// finished.
	pub(super) waker: AtomicWaker,
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
		task: Task,
		/// Name of the thread.
		name: Option<String>,
	},
	/// Terminate thread.
	Terminate {
		/// [`ThreadId`] of the thread to be terminated.
		id: ThreadId,
		/// Value to use `Atomics.waitAsync` on.
		value: Box<i32>,
		/// TLS base address.
		tls_base: f64,
		/// Size of the allocated space.
		stack_alloc: f64,
	},
}

/// Returns the [`ThreadId`] of the current thread without cloning the [`Arc`].
fn current_id() -> ThreadId {
	THREAD.with(|cell| cell.get_or_init(Thread::new).id())
}

/// Initializes the main thread sender and receiver.
fn init_main() {
	SENDER.get_or_init(|| {
		super::has_spawn_support();

		let (sender, receiver) = channel::channel::<Command>();

		wasm_bindgen_futures::spawn_local(async move {
			while let Ok(command) = receiver.next().await {
				match command {
					Command::Spawn { id, task, name } => {
						spawn_internal(id, task, name.as_deref());
					}
					Command::Terminate {
						id,
						value,
						tls_base,
						stack_alloc,
					} => {
						wasm_bindgen_futures::spawn_local(async move {
							WaitAsync::wait(&value, 0).await;

							WORKERS
								.with(|workers| {
									workers
										.borrow_mut()
										.remove(&id)
										.expect("`Worker` to be destroyed not found")
								})
								.terminate();

							thread_local! {
								/// Caches the [`Exports`] object.
								static EXPORTS: Exports = wasm_bindgen::exports().unchecked_into();
								/// Caches the [`GlobalDescriptor`] needed to reconstruct the [`Global`] values.
								static DESCRIPTOR: GlobalDescriptor = {
									let descriptor: GlobalDescriptor = Object::new().unchecked_into();
									descriptor.set_value("i32");
									descriptor
								};
							}

							let (tls_base, stack_alloc) = DESCRIPTOR.with(|descriptor| {
								(
									Global::new(descriptor, &tls_base.into())
										.expect("unexpected invalid `Global` constructor"),
									Global::new(descriptor, &stack_alloc.into())
										.expect("unexpected invalid `Global` constructor"),
								)
							});

							// SAFETY:
							// - We don't get here until we are sure the thread is blocked and can't
							//   be executing Rust code anymore.
							// - This is only done once per thread.
							// - The correct values are attained by the thread and sent when
							//   finished.
							EXPORTS.with(|exports| unsafe {
								exports.thread_destroy(&tls_base, &stack_alloc);
							});
						});
					}
				}
			}
		});

		sender
	});
}

/// Internal spawn function.
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
	let thread = Thread::new_with_name(name);
	let shared = Arc::new(Shared {
		value: Mutex::new(None),
		cvar: Condvar::new(),
		waker: AtomicWaker::new(),
	});

	if let Some(scope) = &scope {
		// This can't overflow because creating a `ThreadId` would fail beforehand.
		scope.threads.fetch_add(1, Ordering::Relaxed);
	}

	let task: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = u32>>> + Send> = Box::new({
		let thread = thread.clone();
		let shared = Arc::downgrade(&shared);
		move || {
			Thread::register(thread);
			let task = task();

			Box::pin(async move {
				let result = task.await;

				if let Some(shared) = shared.upgrade() {
					*shared.value.lock().unwrap_or_else(PoisonError::into_inner) = Some(result);
					shared.cvar.notify_one();
					shared.waker.wake();
				} else {
					drop(result);
				}

				if let Some(scope) = scope {
					if scope.threads.fetch_sub(1, Ordering::Release) == 1 {
						scope.thread.unpark();
						scope.waker.wake();
					}
				}

				let value = Box::new(0);
				let index: *const i32 = value.deref();
				#[allow(clippy::as_conversions)]
				let index = index as u32 / 4;

				let exports: Exports = wasm_bindgen::exports().unchecked_into();
				let tls_base = Number::unchecked_from_js(exports.tls_base().value()).value_of();
				let stack_alloc =
					Number::unchecked_from_js(exports.stack_alloc().value()).value_of();

				SENDER
					.get()
					.expect("closing thread without `SENDER` being initialized")
					.send(Command::Terminate {
						id: current_id(),
						value,
						tls_base,
						stack_alloc,
					})
					.expect("`Receiver` was somehow dropped from the main thread");

				index
			})
		}
	});
	// SAFETY: `scope` has to be `Some`, which prevents this thread from outliving
	// its lifetime.
	let task: TaskStatic = unsafe { mem::transmute(task) };

	if *MAIN_THREAD.get_or_init(current_id) == current_id() {
		init_main();

		spawn_internal(thread.id(), task, thread.name());
	} else {
		SENDER
			.get()
			.expect("not spawning from main thread without initializing `SENDER`")
			.send(Command::Spawn {
				id: thread.id(),
				task,
				name: thread.0.name.clone(),
			})
			.expect("`Receiver` was somehow dropped from the main thread");
	}

	Ok(JoinHandle {
		shared,
		thread,
		taken: AtomicBool::new(false),
	})
}

/// Spawning thread regardless of being nested.
fn spawn_internal(id: ThreadId, task: TaskStatic, name: Option<&str>) {
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

	let message = MEMORY.with(|memory| {
		MODULE.with(|module| Array::of3(module, memory, &Box::into_raw(Box::new(task)).into()))
	});

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
pub async unsafe fn __web_thread_entry(task: *mut Task) -> u32 {
	// SAFETY: Has to be a valid pointer to a `Box<dyn FnOnce() -> u32>`. We only
	// call `__web_thread_entry` from `worker.js`. The data sent to it should
	// only come from `self::spawn_internal()`.
	let task = *unsafe { Box::from_raw(task) };
	task().await
}
