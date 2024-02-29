//! Main thread initialization and command handling.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::AtomicI32;
use std::sync::OnceLock;

use web_sys::Worker;
#[cfg(feature = "message")]
use {wasm_bindgen::closure::Closure, web_sys::MessageEvent};

use super::super::ThreadId;
use super::channel::{self, Sender};
use super::memory::ThreadMemory;
use super::spawn::{self, SpawnData, Task};
use super::wait_async::WaitAsync;

/// [`Sender`] to the main thread.
static SENDER: OnceLock<Sender<Command>> = OnceLock::new();

thread_local! {
	/// Containing all spawned workers.
	pub(super) static WORKERS: RefCell<HashMap<ThreadId, WorkerState>> = RefCell::new(HashMap::new());
}

/// State for each [`Worker`].
pub(super) struct WorkerState {
	/// [`Worker`]
	pub(super) this: Worker,
	/// Callback handling messages.
	#[cfg(feature = "message")]
	pub(super) _message_handler: Closure<dyn Fn(MessageEvent)>,
}

/// Command sent to the main thread.
pub(super) enum Command {
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
	/// Sent by
	/// [`AudioWorkletHandle::destroy()`](super::audio_worklet::AudioWorkletHandle::destroy)
	/// to destroy the thread.
	#[cfg(feature = "audio-worklet")]
	Destroy(ThreadMemory),
}

impl Command {
	/// Sends command to be executed on the main thread.
	pub(super) fn send(self) {
		SENDER
			.get()
			.expect("sending `Command` before `SENDER` is initialized")
			.send(self)
			.expect("`Receiver` was somehow dropped from the main thread");
	}
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

	SENDER.get_or_init(|| {
		super::has_spawn_support();

		let (sender, receiver) = channel::channel::<Command>();

		wasm_bindgen_futures::spawn_local(async move {
			while let Ok(command) = receiver.next().await {
				match command {
					Command::Spawn(SpawnData { id, task, name }) => {
						spawn::spawn_internal(id, task, name.as_deref());
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
					#[cfg(feature = "audio-worklet")]
					Command::Destroy(memory) =>
					// SAFETY: Safety has to be uphold by the caller. See
					// `AudioWorkletHandle::destroy()`.
					unsafe { memory.destroy() },
				}
			}
		});

		sender
	});
}
