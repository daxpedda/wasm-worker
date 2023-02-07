use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{ready, Context, Poll};

use js_sys::{Array, Function};
use pin_project_lite::pin_project;
use wasm_bindgen::closure::Closure as JsClosure;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker};

use crate::Message;

#[derive(Debug)]
pub struct OldMessageHandler(Option<Rc<()>>);

impl OldMessageHandler {
	#[allow(clippy::missing_const_for_fn)]
	pub(super) fn new(closure: Option<Closure>) -> Self {
		if let Some(Closure::Future { running, .. }) = closure {
			if Rc::strong_count(&running) != 1 {
				return Self(Some(running));
			}
		}
		Self(None)
	}

	#[must_use]
	pub fn is_running(&self) -> bool {
		if let Some(running) = &self.0 {
			Rc::strong_count(running) != 1
		} else {
			false
		}
	}
}

#[derive(Debug)]
pub(super) enum Closure {
	Classic(JsClosure<dyn FnMut(web_sys::MessageEvent)>),
	Future {
		running: Rc<()>,
		closure: JsClosure<dyn FnMut(web_sys::MessageEvent) -> JsValue>,
	},
}

impl Deref for Closure {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Classic(closure) => closure.as_ref(),
			Self::Future { closure, .. } => closure.as_ref(),
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
		let running = Rc::new(());

		let closure = JsClosure::new({
			let running = Rc::downgrade(&running);
			move |event| {
				let running = Weak::upgrade(&running).unwrap();
				let closure = &mut closure;

				wasm_bindgen_futures::future_to_promise(Abortable {
					running,
					future: closure(event),
				})
				.into()
			}
		});

		Self::Future { running, closure }
	}
}

pin_project! {
	struct Abortable<F> {
		running: Rc<()>,
		#[pin]
		future: F,
	}
}

impl<F: 'static + Future<Output = ()>> Future for Abortable<F> {
	type Output = Result<JsValue, JsValue>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		if Rc::strong_count(&self.running) != 1 {
			ready!(self.project().future.poll(cx));
		}

		Poll::Ready(Ok(JsValue::UNDEFINED))
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
	) {
		let mut messages = messages
			.into_iter()
			.map(Into::into)
			.map(Message::into_js_value);

		let array = 'array: {
			let Some(message_1) = messages.next() else {
				return
			};

			let Some(message_2) = messages.next() else {
				return self
					.post_message_with_transfer(&message_1, &Array::of1(&message_1))
					.unwrap_throw();
			};

			let Some(message_3) = messages.next() else {
				break 'array  Array::of2(&message_1, &message_2);
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
			.unwrap_throw();
	}
}
