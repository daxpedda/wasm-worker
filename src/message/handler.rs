use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::ops::Deref;

use js_sys::{Function, Promise};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Debug)]
pub(crate) enum MessageHandler {
	Classic(Closure<dyn FnMut(web_sys::MessageEvent)>),
	Future(Closure<dyn FnMut(web_sys::MessageEvent) -> Promise>),
}

impl Deref for MessageHandler {
	type Target = Function;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Classic(closure) => closure.as_ref(),
			Self::Future(closure) => closure.as_ref(),
		}
		.unchecked_ref()
	}
}

impl MessageHandler {
	pub(crate) fn classic(closure: impl 'static + FnMut(web_sys::MessageEvent)) -> Self {
		Self::Classic(Closure::new(closure))
	}

	pub(crate) fn future<F: 'static + Future<Output = ()>>(
		mut closure: impl 'static + FnMut(web_sys::MessageEvent) -> F,
	) -> Self {
		let closure = Closure::new({
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

pub(crate) struct SendMessageHandler<C>(Box<dyn FnOnce(C) -> MessageHandler + Send>);

impl<C> SendMessageHandler<C> {
	pub(crate) fn classic<F: 'static + FnMut(web_sys::MessageEvent)>(
		closure: impl 'static + FnOnce(C) -> F + Send,
	) -> Self {
		Self(Box::new(|context| {
			MessageHandler::classic(closure(context))
		}))
	}

	pub(crate) fn future<
		F1: 'static + FnMut(web_sys::MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		closure: impl 'static + FnOnce(C) -> F1 + Send,
	) -> Self {
		Self(Box::new(|context| MessageHandler::future(closure(context))))
	}

	pub(crate) fn into_message_handler(self, context: C) -> MessageHandler {
		self.0(context)
	}
}

impl<C> Debug for SendMessageHandler<C> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("SendMessageHandler")
			.field(&"Box<FnOnce(C) -> MessageHandler>")
			.finish()
	}
}
