use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::ops::Deref;

use js_sys::WebAssembly::Global;
use js_sys::{Array, Function, Number, Object, Promise, Reflect};
use once_cell::unsync::Lazy;
use wasm_bindgen::closure::Closure as JsClosure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, DomException, Worker};

use crate::message::{Message, Messages, RawMessages};

#[derive(Debug)]
pub(super) enum Closure {
	Classic(JsClosure<dyn FnMut(web_sys::MessageEvent)>),
	Future(JsClosure<dyn FnMut(web_sys::MessageEvent) -> Promise>),
}

impl Deref for Closure {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Classic(closure) => closure.as_ref(),
			Self::Future(closure) => closure.as_ref(),
		}
		.unchecked_ref()
	}
}

impl Closure {
	pub(super) fn classic(closure: impl 'static + FnMut(web_sys::MessageEvent)) -> Self {
		Self::Classic(JsClosure::new(closure))
	}

	pub(super) fn future<F: 'static + Future<Output = ()>>(
		mut closure: impl 'static + FnMut(web_sys::MessageEvent) -> F,
	) -> Self {
		let closure = JsClosure::new({
			move |event| {
				let closure = closure(event);
				wasm_bindgen_futures::future_to_promise(async move {
					closure.await;
					Ok(JsValue::UNDEFINED)
				})
			}
		});

		Self::Future(closure)
	}
}

pub(super) enum WorkerOrContext<'this> {
	Worker(&'this Worker),
	Context(&'this DedicatedWorkerGlobalScope),
}

impl WorkerOrContext<'_> {
	fn post_message_with_transfer(
		self,
		message: &JsValue,
		transfer: &JsValue,
	) -> Result<(), JsValue> {
		match self {
			WorkerOrContext::Worker(worker) => worker.post_message_with_transfer(message, transfer),
			WorkerOrContext::Context(context) => {
				context.post_message_with_transfer(message, transfer)
			}
		}
	}

	pub(super) fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		self,
		messages: M,
	) -> Result<(), TransferError> {
		let mut messages = messages.into_iter().map(Into::into).map(Message::into_raw);

		let array = 'array: {
			let Some(message_1) = messages.next() else {
				return Ok(())
			};

			let Some(message_2) = messages.next() else {
				let array = Array::of1(&message_1);
				return self
					.post_message_with_transfer(&message_1, &array)
					.map_err(|error| TransferError {
						error: error.into(),
						messages: Messages(RawMessages::Single(message_1)),
					});
			};

			let Some(message_3) = messages.next() else {
				break 'array Array::of2(&message_1, &message_2);
			};

			let Some(message_4) = messages.next() else {
				break 'array Array::of3(&message_1, &message_2, &message_3);
			};

			if let Some(message_5) = messages.next() {
				let array = Array::of5(&message_1, &message_2, &message_3, &message_4, &message_5);

				for message in messages {
					array.push(&message);
				}

				array
			} else {
				Array::of4(&message_1, &message_2, &message_3, &message_4)
			}
		};

		self.post_message_with_transfer(&array, &array)
			.map_err(|error| TransferError {
				error: error.into(),
				messages: Messages(RawMessages::Array(array)),
			})
	}
}

thread_local! {
	pub(super) static EXPORTS: Lazy<Exports> = Lazy::new(|| wasm_bindgen::exports().unchecked_into());
}

pub(super) type Exports = __wasm_worker_Exports;

#[wasm_bindgen]
extern "C" {
	#[allow(non_camel_case_types)]
	pub(super) type __wasm_worker_Exports;

	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(super) unsafe fn thread_destroy(
		this: &__wasm_worker_Exports,
		tls_base: &Global,
		stack_alloc: &Global,
	);

	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(super) fn tls_base(this: &__wasm_worker_Exports) -> Global;

	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(super) fn stack_alloc(this: &__wasm_worker_Exports) -> Global;
}

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Tls {
	pub(super) id: usize,
	tls_base: f64,
	stack_alloc: f64,
}

impl Tls {
	thread_local! {
		static DESCRIPTOR: Lazy<Object> = Lazy::new(|| {
			let descriptor = Object::new();
			Reflect::set(&descriptor, &"value".into(), &"i32".into()).unwrap();
			descriptor
		});
	}

	pub(super) fn new(id: usize, tls_base: &Global, stack_alloc: &Global) -> Self {
		let tls_base = Number::unchecked_from_js(tls_base.value()).value_of();
		let stack_alloc = Number::unchecked_from_js(stack_alloc.value()).value_of();

		Self {
			id,
			tls_base,
			stack_alloc,
		}
	}

	pub(super) fn tls_base(&self) -> Global {
		Self::DESCRIPTOR.with(|descriptor| Global::new(descriptor, &self.tls_base.into()).unwrap())
	}

	pub(super) fn stack_alloc(&self) -> Global {
		Self::DESCRIPTOR
			.with(|descriptor| Global::new(descriptor, &self.stack_alloc.into()).unwrap())
	}
}

#[derive(Debug)]
pub struct TransferError {
	pub error: DomException,
	pub messages: Messages,
}

impl Display for TransferError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "error transferring type: {:?}", self.error)
	}
}

impl Error for TransferError {}
