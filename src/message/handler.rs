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
