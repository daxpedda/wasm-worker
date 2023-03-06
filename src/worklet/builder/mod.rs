mod future;

use std::borrow::Cow;
use std::cell::Cell;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use js_sys::Reflect;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, BaseAudioContext};
#[cfg(feature = "message")]
use {
	super::WorkletRef,
	crate::message::{MessageEvent, MessageHandler},
	std::cell::RefCell,
	std::future::Future,
	std::rc::Weak,
};

pub use self::future::WorkletFuture;
use super::{WorkletContext, WorkletUrl, WorkletUrlFuture};

#[must_use = "does nothing unless spawned"]
#[derive(Debug)]
pub struct WorkletBuilder<'url> {
	url: DefaultOrUrl<'url>,
	#[cfg(feature = "message")]
	id: Rc<Cell<Option<u64>>>,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
}

#[derive(Debug)]
enum DefaultOrUrl<'url> {
	Default(WorkletUrlFuture<'static, 'static, true>),
	Url(&'url WorkletUrl),
}

impl WorkletBuilder<'_> {
	pub fn new() -> WorkletBuilder<'static> {
		WorkletBuilder {
			url: DefaultOrUrl::Default(WorkletUrl::default()),
			#[cfg(feature = "message")]
			id: Rc::new(Cell::new(None)),
			#[cfg(feature = "message")]
			message_handler: Rc::new(RefCell::new(None)),
		}
	}

	#[cfg_attr(not(feature = "message"), allow(clippy::missing_const_for_fn))]
	pub fn new_with_url(url: &WorkletUrl) -> WorkletBuilder<'_> {
		WorkletBuilder {
			url: DefaultOrUrl::Url(url),
			#[cfg(feature = "message")]
			id: Rc::new(Cell::new(None)),
			#[cfg(feature = "message")]
			message_handler: Rc::new(RefCell::new(None)),
		}
	}

	#[cfg(feature = "message")]
	pub fn message_handler<F>(self, mut message_handler: F) -> Self
	where
		F: 'static + FnMut(&WorkletRef, MessageEvent),
	{
		let id_handle = Rc::clone(&self.id);
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(MessageHandler::classic({
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkletRef::new(
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
		F1: 'static + FnMut(&WorkletRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(MessageHandler::future({
			let id_handle = Rc::clone(&self.id);
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					WorkletRef::new(
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

	pub fn add<F>(
		self,
		context: &BaseAudioContext,
		f: F,
	) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F: 'static + FnOnce(WorkletContext) + Send,
	{
		let init = Reflect::get(context, &"__wasm_worker_init".into()).unwrap();

		if let Some(init) = init.as_bool() {
			debug_assert!(init);

			return Err(WorkletInitError);
		}

		debug_assert!(init.is_undefined());
		Reflect::set(context, &"__wasm_worker_init".into(), &true.into()).unwrap();

		Ok(WorkletFuture::new(
			Cow::Borrowed(context),
			Box::new(|global, id| {
				let context = WorkletContext::init(global, id);
				f(context);
			}),
			Rc::new(Cell::new(None)),
			#[cfg(feature = "message")]
			Rc::new(RefCell::new(None)),
			self.url,
		))
	}
}

#[doc(hidden)]
#[allow(unreachable_pub)]
pub struct Data {
	id: u64,
	task: Box<dyn 'static + FnOnce(AudioWorkletGlobalScope, u64) + Send>,
}

#[doc(hidden)]
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __wasm_worker_worklet_entry(data: *mut Data) {
	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worklet_entry` from `worklet.js`. The data sent to it should
	// only come from `WorkletFuture::poll()`.
	let data = *unsafe { Box::from_raw(data) };

	let global = js_sys::global().unchecked_into();

	(data.task)(global, data.id);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WorkletInitError;

impl Display for WorkletInitError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "already added a Wasm module to this worklet")
	}
}

impl Error for WorkletInitError {}
