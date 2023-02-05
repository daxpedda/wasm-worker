use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use super::{Close, MessageEvent, ModuleSupportError, WorkerContext, WorkerHandle};
use crate::ScriptUrl;

#[must_use = "does nothing unless spawned"]
pub struct WorkerBuilder<'url, 'name> {
	url: &'url ScriptUrl,
	name: Option<Cow<'name, str>>,
	message_handler: Option<Box<dyn FnMut(web_sys::MessageEvent)>>,
}

impl Debug for WorkerBuilder<'_, '_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("WorkerBuilder")
			.field("url", &self.url)
			.field("name", &self.name)
			.field(
				"closure",
				&if self.message_handler.is_some() {
					"Some"
				} else {
					"None"
				},
			)
			.finish()
	}
}

impl WorkerBuilder<'_, '_> {
	pub fn new() -> Result<WorkerBuilder<'static, 'static>, ModuleSupportError> {
		Self::new_with_url(crate::default_script_url())
	}

	pub fn new_with_url(url: &ScriptUrl) -> Result<WorkerBuilder<'_, 'static>, ModuleSupportError> {
		if url.is_module() && !Self::has_module_support() {
			return Err(ModuleSupportError);
		}

		Ok(WorkerBuilder {
			url,
			name: None,
			message_handler: None,
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

	pub fn clear_message_handler(mut self) -> Self {
		self.message_handler.take();
		self
	}

	pub fn set_message_handler<F: 'static + FnMut(MessageEvent)>(
		mut self,
		mut message_handler: F,
	) -> Self {
		self.message_handler.replace(Box::new(move |event| {
			message_handler(MessageEvent::new(event));
		}));
		self
	}

	pub fn spawn<F1, F2>(self, f: F1) -> WorkerHandle
	where
		F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
		F2: 'static + Future<Output = Close>,
	{
		let work = Task(Box::new(move |context| {
			Box::pin(async move { f(context).await })
		}));
		let task = Box::into_raw(Box::new(work));

		let mut options = None;

		if let Some(name) = self.name {
			options.get_or_insert_with(WorkerOptions::new).name(&name);
		}

		if self.url.is_module() {
			options
				.get_or_insert_with(WorkerOptions::new)
				.type_(WorkerType::Module);
		}

		let worker = if let Some(options) = options {
			Worker::new_with_options(&self.url.url, &options)
		} else {
			Worker::new(&self.url.url)
		}
		.unwrap_throw();

		let closure = self.message_handler.map(Closure::new);

		worker.set_onmessage(
			closure
				.as_ref()
				.map(Closure::as_ref)
				.map(JsCast::unchecked_ref),
		);

		let init = Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&task.into(),
		);

		worker.post_message(&init).unwrap_throw();

		WorkerHandle { worker, closure }
	}
}

impl<'name> WorkerBuilder<'_, 'name> {
	pub fn url<'url>(
		self,
		url: &'url ScriptUrl,
	) -> Result<WorkerBuilder<'url, 'name>, ModuleSupportError> {
		if url.is_module() && !Self::has_module_support() {
			return Err(ModuleSupportError);
		}

		Ok(WorkerBuilder {
			url,
			name: self.name,
			message_handler: self.message_handler,
		})
	}
}

impl<'url> WorkerBuilder<'url, '_> {
	pub fn clear_name(self) -> WorkerBuilder<'url, 'static> {
		WorkerBuilder {
			url: self.url,
			name: None,
			message_handler: self.message_handler,
		}
	}

	pub fn name<'name, N: Into<Cow<'name, str>>>(self, name: N) -> WorkerBuilder<'url, 'name> {
		WorkerBuilder {
			url: self.url,
			name: Some(name.into()),
			message_handler: self.message_handler,
		}
	}
}

#[doc(hidden)]
#[allow(
	clippy::type_complexity,
	missing_debug_implementations,
	unreachable_pub
)]
pub struct Task(
	Box<
		dyn 'static
			+ FnOnce(WorkerContext) -> Pin<Box<dyn 'static + Future<Output = Close>>>
			+ Send,
	>,
);

#[doc(hidden)]
#[allow(unreachable_pub)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(task: *mut Task) -> bool {
	// Unhooking the message handler has to happen in JS because loading the WASM
	// module will actually yield and introduces a race condition where messages
	// sent will still be handled by the starter message handler.

	// SAFETY: The argument is an address that has to be a valid pointer to a
	// `Task`.
	let Task(work) = *unsafe { Box::from_raw(task) };

	let context = WorkerContext(js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>());
	let close = work(context).await;

	close.to_bool()
}
