use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{ready, Context, Poll, Waker};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use js_sys::Boolean;
use once_cell::sync::OnceCell;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use crate::common::WAIT_ASYNC_SUPPORT;
use crate::global::{Global, GlobalContext};

static SUPPORT: OnceCell<bool> = OnceCell::new();

pub fn has_async_support() -> Result<AsyncSupportFuture, AsyncSupportError> {
	if let Some(support) = SUPPORT.get() {
		return Ok(AsyncSupportFuture(Some(State::Ready(*support))));
	}

	let state = if *WAIT_ASYNC_SUPPORT {
		State::Ready(true)
	} else {
		GlobalContext::with(|global| match global {
			GlobalContext::Window(_) => {
				let worker = web_sys::Worker::new(
					"data:,postMessage%28typeof%20Worker%21%3D%3D%27undefined%27%29",
				)
				.unwrap();
				let oneshot = Oneshot::new();
				let closure = Closure::new({
					let oneshot = oneshot.clone();
					move |event: MessageEvent| {
						let data: Boolean = event.data().unchecked_into();
						oneshot.set(data.value_of());
					}
				});
				worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));

				Ok(State::Worker {
					worker,
					_message_handler: closure,
					oneshot,
				})
			}
			GlobalContext::Worker(_) => Ok(State::Ready(Global::has_worker())),
			GlobalContext::Worklet => Err(AsyncSupportError),
		})?
	};

	if let State::Ready(support) = state {
		if let Err((old_support, _)) = SUPPORT.try_insert(support) {
			debug_assert_eq!(
				support, *old_support,
				"determining support has yielded different results"
			);
		}
	}

	Ok(AsyncSupportFuture(Some(state)))
}

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct AsyncSupportFuture(Option<State>);

#[derive(Debug)]
enum State {
	Ready(bool),
	Worker {
		worker: web_sys::Worker,
		_message_handler: Closure<dyn Fn(MessageEvent)>,
		oneshot: Oneshot,
	},
}

impl AsyncSupportFuture {
	#[allow(clippy::wrong_self_convention)]
	pub fn into_inner(&mut self) -> Option<bool> {
		let state = self.0.as_ref().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let Some(new_support) = self.abort() {
				debug_assert_eq!(
					*support, new_support,
					"determining support has yielded different results"
				);
			}

			return Some(*support);
		}

		match state {
			State::Ready(support) => {
				let support = *support;
				self.0.take();

				Some(support)
			}
			State::Worker {
				worker, oneshot, ..
			} => {
				if let Some(support) = oneshot.get() {
					worker.terminate();
					worker.set_onmessage(None);

					Some(support)
				} else {
					None
				}
			}
		}
	}

	fn abort(&mut self) -> Option<bool> {
		match self.0.take()? {
			State::Ready(support) => Some(support),
			State::Worker {
				worker, oneshot, ..
			} => {
				worker.terminate();
				worker.set_onmessage(None);

				oneshot.get()
			}
		}
	}
}

impl Drop for AsyncSupportFuture {
	fn drop(&mut self) {
		self.abort();
	}
}

impl Future for AsyncSupportFuture {
	type Output = bool;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let state = self.0.as_mut().expect("polled after `Ready`");

		if let Some(support) = SUPPORT.get() {
			if let Some(new_support) = self.abort() {
				debug_assert_eq!(
					*support, new_support,
					"determining support has yielded different results"
				);
			}

			return Poll::Ready(*support);
		}

		match state {
			State::Ready(support) => {
				let support = *support;
				self.0.take();

				Poll::Ready(support)
			}
			State::Worker {
				worker, oneshot, ..
			} => {
				let support = ready!(Pin::new(oneshot).poll(cx));
				worker.terminate();
				self.0.take();

				if let Err((old_support, _)) = SUPPORT.try_insert(support) {
					debug_assert_eq!(
						support, *old_support,
						"determining support has yielded different results"
					);
				}

				Poll::Ready(support)
			}
		}
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for AsyncSupportFuture {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncSupportError;

impl Display for AsyncSupportError {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		write!(formatter, "context can't be used to determine support")
	}
}

impl Error for AsyncSupportError {}

#[derive(Clone, Debug)]
struct Oneshot(Rc<Inner>);

#[derive(Debug)]
struct Inner {
	waker: RefCell<Option<Waker>>,
	result: Cell<Option<bool>>,
}

impl Oneshot {
	fn new() -> Self {
		Self(Rc::new(Inner {
			waker: RefCell::default(),
			result: Cell::default(),
		}))
	}

	fn get(&self) -> Option<bool> {
		self.0.result.get()
	}

	fn set(&self, result: bool) {
		self.0.result.set(Some(result));

		if let Some(waker) = self.0.waker.borrow_mut().take() {
			waker.wake();
		}
	}
}

impl Future for Oneshot {
	type Output = bool;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		// Short-circuit.
		if let Some(result) = self.0.result.get() {
			return Poll::Ready(result);
		}

		*self.0.waker.borrow_mut() = Some(cx.waker().clone());

		if let Some(result) = self.0.result.get() {
			Poll::Ready(result)
		} else {
			Poll::Pending
		}
	}
}
