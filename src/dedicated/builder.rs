use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, WorkerOptions, WorkerType};

use super::{Closure, Worker, WorkerContext, WorkerRef, WorkerUrl};
use crate::message::MessageEvent;

#[must_use = "does nothing unless spawned"]
#[derive(Debug)]
pub struct WorkerBuilder<'url> {
	url: &'url WorkerUrl,
	options: Option<WorkerOptions>,
	id: Rc<Cell<Option<usize>>>,
	message_handler: Rc<RefCell<Option<Closure>>>,
}

impl WorkerBuilder<'_> {
	pub fn new() -> Result<WorkerBuilder<'static>, ModuleSupportError> {
		Self::new_with_url(WorkerUrl::default())
	}

	pub fn new_with_url(url: &WorkerUrl) -> Result<WorkerBuilder<'_>, ModuleSupportError> {
		#[allow(clippy::if_then_some_else_none)]
		let options = if url.is_module() {
			if !Self::has_module_support() {
				return Err(ModuleSupportError);
			}

			let mut options = WorkerOptions::new();
			options.type_(WorkerType::Module);
			Some(options)
		} else {
			None
		};

		Ok(WorkerBuilder {
			url,
			options,
			id: Rc::new(Cell::new(None)),
			message_handler: Rc::new(RefCell::new(None)),
		})
	}

	#[must_use]
	pub fn has_module_support() -> bool {
		static HAS_MODULE_SUPPORT: Lazy<bool> = Lazy::new(|| {
			#[wasm_bindgen]
			#[allow(non_camel_case_types)]
			struct __wasm_worker_Tester(Rc<Cell<bool>>);

			#[wasm_bindgen]
			impl __wasm_worker_Tester {
				#[allow(unreachable_pub)]
				#[wasm_bindgen(getter = type)]
				pub fn type_(&self) {
					self.0.set(true);
				}
			}

			let tester = Rc::new(Cell::new(false));
			let worker_options =
				WorkerOptions::from(JsValue::from(__wasm_worker_Tester(Rc::clone(&tester))));
			let worker =
				web_sys::Worker::new_with_options("data:,", &worker_options).unwrap_throw();
			worker.terminate();

			tester.get()
		});

		*HAS_MODULE_SUPPORT
	}

	pub fn name(mut self, name: &str) -> Self {
		self.options
			.get_or_insert_with(WorkerOptions::new)
			.name(name);
		self
	}

	pub fn message_handler<F: 'static + FnMut(&WorkerRef, MessageEvent)>(
		self,
		mut message_handler: F,
	) -> Self {
		let id_handle = Rc::clone(&self.id);
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(Closure::classic({
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

	pub fn message_handler_async<
		F1: 'static + FnMut(&WorkerRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		self,
		mut message_handler: F1,
	) -> Self {
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(Closure::future({
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

	pub fn spawn<F>(self, f: F) -> Worker
	where
		F: 'static + FnOnce(WorkerContext) + Send,
	{
		self.spawn_internal(Task::Classic(Box::new(f)))
	}

	pub fn spawn_async<F1, F2>(self, f: F1) -> Worker
	where
		F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		let task = Task::Future(Box::new(move |context| {
			Box::pin(async move {
				f(context).await;
				Ok(JsValue::UNDEFINED)
			})
		}));

		self.spawn_internal(task)
	}

	fn spawn_internal(self, task: Task) -> Worker {
		static COUNTER: AtomicUsize = AtomicUsize::new(0);

		let id = COUNTER.fetch_add(1, Ordering::Relaxed);
		self.id.set(Some(id));

		let data = Box::into_raw(Box::new(Data { id, task }));

		let worker = if let Some(options) = self.options {
			web_sys::Worker::new_with_options(&self.url.url, &options)
		} else {
			web_sys::Worker::new(&self.url.url)
		}
		.unwrap_throw();

		if let Some(message_handler) = RefCell::borrow(&self.message_handler).deref() {
			worker.set_onmessage(Some(message_handler));
		}

		let init = Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&data.into(),
		);

		worker.post_message(&init).unwrap_throw();

		Worker::new(worker, self.id, self.message_handler)
	}
}

#[doc(hidden)]
#[allow(unreachable_pub)]
pub struct Data {
	id: usize,
	task: Task,
}

#[allow(clippy::type_complexity)]
enum Task {
	Classic(Box<dyn 'static + FnOnce(WorkerContext) + Send>),
	Future(
		Box<
			dyn 'static
				+ FnOnce(
					WorkerContext,
				) -> Pin<Box<dyn 'static + Future<Output = Result<JsValue, JsValue>>>>
				+ Send,
		>,
	),
}

#[doc(hidden)]
#[allow(unreachable_pub)]
#[wasm_bindgen]
pub unsafe fn __wasm_worker_dedicated_entry(data: *mut Data) -> JsValue {
	let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();
	global.set_onmessage(None);

	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_dedicated_entry` from `worker.js`. The data sent to it should
	// only come from `WorkerBuilder::spawn_internal()`.
	let data = *unsafe { Box::from_raw(data) };

	let context = WorkerContext::init(global, data.id);

	match data.task {
		Task::Classic(classic) => {
			classic(context);
			JsValue::UNDEFINED
		}
		Task::Future(future) => wasm_bindgen_futures::future_to_promise(future(context)).into(),
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSupportError;

impl Display for ModuleSupportError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "browser doesn't support worker modules")
	}
}

impl Error for ModuleSupportError {}
