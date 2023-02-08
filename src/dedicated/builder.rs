use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::{Rc, Weak};

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use super::{Closure, MessageEvent, WorkerContext, WorkerHandle, WorkerHandleRef};
use crate::WorkerUrl;

#[must_use = "does nothing unless spawned"]
#[derive(Debug)]
pub struct WorkerBuilder<'url> {
	url: &'url WorkerUrl,
	options: Option<WorkerOptions>,
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
			message_handler: Rc::new(RefCell::new(None)),
		})
	}

	#[must_use]
	pub fn has_module_support() -> bool {
		static HAS_MODULE_SUPPORT: Lazy<bool> = Lazy::new(|| {
			#[wasm_bindgen]
			struct Tester(Rc<Cell<bool>>);

			#[wasm_bindgen]
			impl Tester {
				#[allow(unreachable_pub)]
				#[wasm_bindgen(getter = type)]
				pub fn type_(&self) {
					self.0.set(true);
				}
			}

			let tester = Rc::new(Cell::new(false));
			let worker_options = WorkerOptions::from(JsValue::from(Tester(Rc::clone(&tester))));
			let worker = Worker::new_with_options("data:,", &worker_options).unwrap_throw();
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

	pub fn set_message_handler<F: 'static + FnMut(WorkerHandleRef, MessageEvent)>(
		self,
		mut message_handler: F,
	) -> Self {
		let message_handler_holder = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(Closure::classic({
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkerHandleRef::new(
						event.target().unwrap().unchecked_into(),
						Weak::clone(&message_handler_holder),
					)
				});
				message_handler(handle.clone(), MessageEvent::new(event));
			}
		}));
		self
	}

	pub fn set_message_handler_async<
		F1: 'static + FnMut(WorkerHandleRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		self,
		mut message_handler: F1,
	) -> Self {
		let message_handler_holder = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(Closure::future({
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkerHandleRef::new(
						event.target().unwrap().unchecked_into(),
						Weak::clone(&message_handler_holder),
					)
				});
				message_handler(handle.clone(), MessageEvent::new(event))
			}
		}));
		self
	}

	pub fn spawn<F>(self, f: F) -> WorkerHandle
	where
		F: 'static + FnMut(WorkerContext) -> Close + Send,
	{
		self.spawn_internal(Task::Classic(Box::new(f)))
	}

	pub fn spawn_async<F1, F2>(self, f: F1) -> WorkerHandle
	where
		F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
		F2: 'static + Future<Output = Close>,
	{
		let task = Task::Future(Box::new(move |context| {
			Box::pin(async move { Ok(f(context).await.to_bool().into()) })
		}));

		self.spawn_internal(task)
	}

	fn spawn_internal(self, task: Task) -> WorkerHandle {
		let task = Box::into_raw(Box::new(task));

		let worker = if let Some(options) = self.options {
			Worker::new_with_options(&self.url.url, &options)
		} else {
			Worker::new(&self.url.url)
		}
		.unwrap_throw();

		if let Some(message_handler) = RefCell::borrow(&self.message_handler).deref() {
			worker.set_onmessage(Some(message_handler));
		}

		let init = Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&task.into(),
		);

		worker.post_message(&init).unwrap_throw();

		WorkerHandle::new(worker, self.message_handler)
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Close {
	Yes,
	No,
}

impl Close {
	const fn to_bool(self) -> bool {
		match self {
			Self::Yes => true,
			Self::No => false,
		}
	}
}

#[doc(hidden)]
#[allow(
	clippy::type_complexity,
	missing_debug_implementations,
	unreachable_pub
)]
pub enum Task {
	Classic(Box<dyn 'static + FnMut(WorkerContext) -> Close>),
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
pub fn __wasm_worker_entry(task: *mut Task) -> JsValue {
	// Unhooking the message handler has to happen in JS because loading the WASM
	// module will actually yield and introduce a race condition where messages sent
	// will still be handled by the entry message handler.

	let context = WorkerContext(js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>());

	// SAFETY: The argument is an address that has to be a valid pointer to a
	// `Task`.
	match *unsafe { Box::from_raw(task) } {
		Task::Classic(mut classic) => classic(context).to_bool().into(),
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

impl From<ModuleSupportError> for JsValue {
	fn from(value: ModuleSupportError) -> Self {
		value.to_string().into()
	}
}
