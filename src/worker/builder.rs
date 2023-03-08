use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, WorkerOptions, WorkerType};
#[cfg(feature = "message")]
use {
	super::WorkerRef,
	crate::message::{
		Message, MessageEvent, MessageHandler, Messages, RawMessages, SendMessageHandler,
		TransferError,
	},
	std::cell::RefCell,
	std::ops::Deref,
	std::rc::Weak,
};

use super::{ModuleSupportError, Worker, WorkerContext, WorkerUrl};
use crate::common::ID_COUNTER;

#[must_use = "does nothing unless spawned"]
#[derive(Debug)]
pub struct WorkerBuilder<'url> {
	url: &'url WorkerUrl,
	options: Option<WorkerOptions>,
	id: Rc<Cell<Result<u64, u64>>>,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
	#[cfg(feature = "message")]
	worker_message_handler: Option<SendMessageHandler<WorkerContext>>,
}

impl WorkerBuilder<'_> {
	pub fn new() -> Result<WorkerBuilder<'static>, ModuleSupportError> {
		WorkerUrl::default().map(Self::new_with_url)
	}

	pub fn new_with_url(url: &WorkerUrl) -> WorkerBuilder<'_> {
		let options = if url.is_module() {
			let mut options = WorkerOptions::new();
			options.type_(WorkerType::Module);
			Some(options)
		} else {
			None
		};

		WorkerBuilder {
			url,
			options,
			id: Rc::new(Cell::new(Err(0))),
			#[cfg(feature = "message")]
			message_handler: Rc::new(RefCell::new(None)),
			#[cfg(feature = "message")]
			worker_message_handler: None,
		}
	}

	pub fn name(mut self, name: &str) -> Self {
		self.options
			.get_or_insert_with(WorkerOptions::new)
			.name(name);
		self
	}

	#[cfg(feature = "message")]
	pub fn message_handler<F>(self, mut message_handler: F) -> Self
	where
		F: 'static + FnMut(&WorkerRef, MessageEvent),
	{
		let id_handle = Rc::clone(&self.id);
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(MessageHandler::function({
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkerRef::new(
						event.target().unwrap().unchecked_into(),
						Rc::clone(&id_handle),
						Weak::clone(&message_handler_handle),
					)
				});
				message_handler(handle, MessageEvent::new(event));
			}
		}));
		self
	}

	#[cfg(feature = "message")]
	pub fn message_handler_async<F1, F2>(self, mut message_handler: F1) -> Self
	where
		F1: 'static + FnMut(&WorkerRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(MessageHandler::future({
			let id_handle = Rc::clone(&self.id);
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkerRef::new(
						event.target().unwrap().unchecked_into(),
						Rc::clone(&id_handle),
						Weak::clone(&message_handler_handle),
					)
				});
				message_handler(handle, MessageEvent::new(event))
			}
		}));
		self
	}

	#[cfg(feature = "message")]
	pub fn worker_message_handler<F>(mut self, mut message_handler: F) -> Self
	where
		F: 'static + FnMut(&WorkerContext, MessageEvent) + Send,
	{
		self.worker_message_handler = Some(SendMessageHandler::function(|context| {
			move |event: web_sys::MessageEvent| {
				message_handler(&context, MessageEvent::new(event));
			}
		}));
		self
	}

	#[cfg(feature = "message")]
	pub fn worker_message_handler_async<F1, F2>(mut self, mut message_handler: F1) -> Self
	where
		F1: 'static + FnMut(&WorkerContext, MessageEvent) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		self.worker_message_handler = Some(SendMessageHandler::future(|context| {
			move |event: web_sys::MessageEvent| message_handler(&context, MessageEvent::new(event))
		}));
		self
	}

	pub fn spawn<F>(self, f: F) -> Worker
	where
		F: 'static + FnOnce(WorkerContext) + Send,
	{
		self.spawn_internal(
			Task::Function(Box::new(f)),
			#[cfg(feature = "message")]
			None,
		)
		.unwrap()
	}

	#[cfg(feature = "message")]
	pub fn spawn_with_message<F, I, M>(self, f: F, messages: I) -> Result<Worker, TransferError>
	where
		F: 'static + FnOnce(WorkerContext, Messages) + Send,
		I: IntoIterator<Item = M>,
		M: Into<Message>,
	{
		let messages = RawMessages::from_messages(messages);
		self.spawn_internal(Task::FunctionWithMessage(Box::new(f)), Some(&messages))
			.map_err(|error| TransferError {
				error: error.into(),
				messages: Messages(messages),
			})
	}

	pub fn spawn_async<F1, F2>(self, f: F1) -> Worker
	where
		F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		let task = Task::Future(Box::new(move |context| {
			Box::pin(async move { f(context).await })
		}));

		self.spawn_internal(
			task,
			#[cfg(feature = "message")]
			None,
		)
		.unwrap()
	}

	#[cfg(feature = "message")]
	pub fn spawn_async_with_message<F1, F2, I, M>(
		self,
		f: F1,
		messages: I,
	) -> Result<Worker, TransferError>
	where
		F1: 'static + FnOnce(WorkerContext, Messages) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
		I: IntoIterator<Item = M>,
		M: Into<Message>,
	{
		let messages = RawMessages::from_messages(messages);
		let task = Task::FutureWithMessage(Box::new(move |context, messages| {
			Box::pin(async move { f(context, messages).await })
		}));

		self.spawn_internal(task, Some(&messages))
			.map_err(|error| TransferError {
				error: error.into(),
				messages: Messages(messages),
			})
	}

	#[cfg_attr(not(feature = "message"), allow(clippy::unnecessary_wraps))]
	fn spawn_internal(
		self,
		task: Task,
		#[cfg(feature = "message")] messages: Option<&RawMessages>,
	) -> Result<Worker, JsValue> {
		let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
		self.id.set(Ok(id));

		let data = Box::into_raw(Box::new(Data {
			id,
			task,
			#[cfg(feature = "message")]
			message_handler: self.worker_message_handler,
		}));

		let worker = if let Some(options) = self.options {
			web_sys::Worker::new_with_options(&self.url.url, &options)
		} else {
			web_sys::Worker::new(&self.url.url)
		}
		.unwrap();

		#[cfg(feature = "message")]
		if let Some(message_handler) = RefCell::borrow(&self.message_handler).deref() {
			worker.set_onmessage(Some(message_handler));
		}

		#[cfg(feature = "message")]
		{
			let result = match messages {
				None | Some(RawMessages::None) => {
					let init = Array::of4(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
						&JsValue::UNDEFINED,
					);

					worker.post_message(&init).unwrap();
					Ok(())
				}
				Some(RawMessages::Single(message)) => {
					let init = Array::of4(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
						message,
					);
					let transfer = Array::of1(message);

					worker.post_message_with_transfer(&init, &transfer)
				}
				Some(RawMessages::Array(messages)) => {
					let init = Array::of4(
						&wasm_bindgen::module(),
						&wasm_bindgen::memory(),
						&data.into(),
						messages,
					);

					worker.post_message_with_transfer(&init, messages)
				}
			};

			if let Err(error) = result {
				// SAFETY: We just wraped this above.
				drop(unsafe { Box::from_raw(data) });

				return Err(error);
			}
		}

		#[cfg(not(feature = "message"))]
		{
			let init = Array::of4(
				&wasm_bindgen::module(),
				&wasm_bindgen::memory(),
				&data.into(),
				&JsValue::UNDEFINED,
			);

			worker.post_message(&init).unwrap();
		}

		Ok(Worker::new(
			worker,
			self.id,
			#[cfg(feature = "message")]
			self.message_handler,
		))
	}
}

#[doc(hidden)]
#[allow(unreachable_pub)]
pub struct Data
where
	Self: Send,
{
	id: u64,
	task: Task,
	#[cfg(feature = "message")]
	message_handler: Option<SendMessageHandler<WorkerContext>>,
}

#[allow(clippy::type_complexity)]
enum Task {
	Function(Box<dyn 'static + FnOnce(WorkerContext) + Send>),
	#[cfg(feature = "message")]
	FunctionWithMessage(Box<dyn 'static + FnOnce(WorkerContext, Messages) + Send>),
	Future(
		Box<
			dyn 'static
				+ FnOnce(WorkerContext) -> Pin<Box<dyn 'static + Future<Output = ()>>>
				+ Send,
		>,
	),
	#[cfg(feature = "message")]
	FutureWithMessage(
		Box<
			dyn 'static
				+ FnOnce(WorkerContext, Messages) -> Pin<Box<dyn 'static + Future<Output = ()>>>
				+ Send,
		>,
	),
}

#[doc(hidden)]
#[wasm_bindgen]
#[allow(unreachable_pub)]
#[cfg_attr(not(feature = "message"), allow(clippy::needless_pass_by_value))]
pub unsafe fn __wasm_worker_worker_entry(
	data: *mut Data,
	#[cfg_attr(not(feature = "message"), allow(unused_variables))] messages: JsValue,
) {
	let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
	#[cfg(not(feature = "message"))]
	global.set_onmessage(None);

	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worker_entry` from `worker.js`. The data sent to it should
	// only come from `WorkerBuilder::spawn_internal()`.
	let data = *unsafe { Box::from_raw(data) };

	let context = WorkerContext::init(
		global,
		data.id,
		#[cfg(feature = "message")]
		data.message_handler,
	);

	match data.task {
		Task::Function(f) => {
			f(context);
		}
		#[cfg(feature = "message")]
		Task::FunctionWithMessage(f) => {
			let messages = Messages(RawMessages::from_js(messages));

			f(context, messages);
		}
		Task::Future(future) => wasm_bindgen_futures::spawn_local(future(context)),
		#[cfg(feature = "message")]
		Task::FutureWithMessage(future) => {
			let messages = Messages(RawMessages::from_js(messages));

			wasm_bindgen_futures::spawn_local(future(context, messages));
		}
	}
}
