//! Handling message related functionality.

use std::cell::RefCell;
use std::future::Future;
use std::sync::Arc;
use std::{io, mem};

use js_sys::Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{MessageEvent, Worker};

use super::super::super::global::Global;
#[cfg(feature = "audio-worklet")]
use super::super::audio_worklet::register::THREAD_LOCK_INDEXES;
use super::super::{channel, main, oneshot, JoinHandle, ScopeData, ThreadId};
use super::{SpawnData, Task};
use crate::thread::atomics::channel::Receiver;
use crate::web::message::{MessageSend, RawMessageSerialize};

thread_local! {
	pub(super) static SPAWN_SENDER: RefCell<Option<channel::Sender<SpawnData>>> = const { RefCell::new(None) };
}

/// Internal spawn function.
///
/// # Safety
///
/// `task` and `message` have to outlive the thread.
pub(in super::super) unsafe fn spawn<F1, F2, T, M>(
	task: F1,
	name: Option<String>,
	scope: Option<Arc<ScopeData>>,
	message: M,
) -> io::Result<JoinHandle<T>>
where
	F1: FnOnce(M) -> F2 + Send,
	F2: Future<Output = T>,
	T: Send,
	M: MessageSend,
{
	let thread = super::thread_init(name, scope.as_deref());
	let (result_sender, result_receiver) = oneshot::channel();
	let (spawn_sender, spawn_receiver) = channel::channel();

	let raw_message = message.send();

	let task: Task<'_> = Box::new({
		let thread = thread.clone();
		move |message| {
			super::thread_runner(thread, result_sender, spawn_sender, scope, move || {
				let message = (!message.is_undefined()).then_some(message);
				let message = M::receive(message, raw_message.send);
				task(message)
			})
		}
	});

	if let Some(serialize) = raw_message.serialize {
		if super::super::is_main_thread() {
			main::init_main_thread();

			spawn_internal(
				thread.id(),
				thread.name(),
				spawn_receiver,
				serialize,
				Box::new(task),
			)?;
		} else {
			// SAFETY: `task` has to be `'static` or `scope` has to be `Some`, which
			// prevents this thread from outliving its lifetime.
			let task = unsafe { mem::transmute::<Task<'_>, Task<'static>>(task) };

			let data = SpawnData {
				id: thread.id(),
				name: thread.0.name.clone(),
				spawn_receiver,
				task,
			};

			SPAWN_SENDER
				.with(|cell| {
					cell.borrow()
						.as_ref()
						.expect("found no `Sender` in existing thread")
						.send(data)
				})
				.expect("`Receiver` in main thread dropped");

			Global::with(|global| match global {
				Global::Dedicated(global) => {
					let result = match serialize {
						RawMessageSerialize {
							serialize,
							transfer: None,
						} => global.post_message(&Array::of1(&serialize)),
						RawMessageSerialize {
							serialize,
							transfer: Some(transfer),
						} => global.post_message_with_transfer(
							&Array::of2(&serialize, &transfer),
							&transfer,
						),
					};

					if let Err(error) = result {
						global.post_message(&JsValue::UNDEFINED).expect(
							"`DedicatedWorkerGlobalScope.postMessage()` is not expected to fail \
							 without a `transfer` object",
						);
						Err(super::super::error_from_exception(error))
					} else {
						Ok(())
					}
				}
				#[allow(clippy::unimplemented)]
				Global::Worklet => {
					unimplemented!("spawning threads with messages from audio worklets")
				}
				_ => unreachable!("spawning from thread not registered by `web-thread`"),
			})?;
		}

		Ok(JoinHandle {
			receiver: Some(result_receiver),
			thread,
		})
	} else {
		Ok(super::spawn_without_message(
			thread,
			result_receiver,
			spawn_receiver,
			task,
		))
	}
}

/// Spawning thread regardless of being nested.
fn spawn_internal(
	id: ThreadId,
	name: Option<&str>,
	spawn_receiver: Receiver<SpawnData>,
	serialize: RawMessageSerialize,
	task: Task<'_>,
) -> io::Result<()> {
	let result = super::spawn_common(
		id,
		name,
		spawn_receiver,
		task,
		#[cfg(not(feature = "audio-worklet"))]
		|worker: &Worker, module, memory, task| match serialize {
			RawMessageSerialize {
				serialize,
				transfer: None,
			} => worker.post_message(&Array::of4(module, memory, &task, &serialize)),
			RawMessageSerialize {
				serialize,
				transfer: Some(transfer),
			} => worker.post_message_with_transfer(
				&Array::of4(module, memory, &task, &serialize),
				&transfer,
			),
		},
		#[cfg(feature = "audio-worklet")]
		|worker: &Worker, module, memory, task| {
			THREAD_LOCK_INDEXES.with(|indexes| match serialize {
				RawMessageSerialize {
					serialize,
					transfer: None,
				} => worker.post_message(&Array::of5(module, memory, indexes, &task, &serialize)),
				RawMessageSerialize {
					serialize,
					transfer: Some(transfer),
				} => worker.post_message_with_transfer(
					&Array::of5(module, memory, indexes, &task, &serialize),
					&transfer,
				),
			})
		},
	);

	if let Err(error) = result {
		Err(super::super::error_from_exception(error))
	} else {
		Ok(())
	}
}

/// Setup `message` event handler.
pub(super) fn setup_message_handler(
	worker: &Worker,
	spawn_receiver: Receiver<SpawnData>,
) -> Closure<dyn Fn(MessageEvent)> {
	let message_handler = Closure::new(move |event: MessageEvent| {
		let data = spawn_receiver
			.try_recv()
			.expect("expected data to have been sent before message");
		let message = event.data();

		if message.is_undefined() {
			return;
		}

		let mut values = message.unchecked_into::<Array>().into_iter();
		let serialize = RawMessageSerialize {
			serialize: values.next().expect("no serialized data found"),
			transfer: values.next().map(Array::unchecked_from_js),
		};

		spawn_internal(
			data.id,
			data.name.as_deref(),
			data.spawn_receiver,
			serialize,
			Box::new(data.task),
		)
		.expect("unexpected serialization error when serialization succeeded when sending this");
	});
	worker.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));

	message_handler
}
