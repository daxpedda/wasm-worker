mod future;

use std::borrow::Cow;
use std::cell::Cell;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use js_sys::Reflect;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletProcessor, BaseAudioContext};
#[cfg(feature = "message")]
use {
	super::WorkletRef,
	crate::message::{MessageEvent, MessageHandler, SendMessageHandler},
	std::cell::RefCell,
	std::rc::Weak,
	web_sys::AudioWorkletNode,
};

pub use self::future::WorkletFuture;
use super::{WorkletContext, WorkletUrl, WorkletUrlFuture};

#[must_use = "does nothing unless spawned"]
#[derive(Debug)]
pub struct WorkletBuilder<'url> {
	url: DefaultOrUrl<'url>,
	id: Rc<Cell<Result<u64, u64>>>,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
	#[cfg(feature = "message")]
	worklet_message_handler: Option<SendMessageHandler<WorkletContext>>,
}

#[derive(Debug)]
enum DefaultOrUrl<'url> {
	Default(WorkletUrlFuture<'static, true>),
	Url(&'url WorkletUrl),
}

impl WorkletBuilder<'_> {
	pub fn new() -> WorkletBuilder<'static> {
		WorkletBuilder {
			url: DefaultOrUrl::Default(WorkletUrl::default()),
			id: Rc::new(Cell::new(Err(0))),
			#[cfg(feature = "message")]
			message_handler: Rc::new(RefCell::new(None)),
			#[cfg(feature = "message")]
			worklet_message_handler: None,
		}
	}

	#[cfg_attr(not(feature = "message"), allow(clippy::missing_const_for_fn))]
	pub fn new_with_url(url: &WorkletUrl) -> WorkletBuilder<'_> {
		WorkletBuilder {
			url: DefaultOrUrl::Url(url),
			id: Rc::new(Cell::new(Err(0))),
			#[cfg(feature = "message")]
			message_handler: Rc::new(RefCell::new(None)),
			#[cfg(feature = "message")]
			worklet_message_handler: None,
		}
	}

	#[cfg(feature = "message")]
	pub fn message_handler<F>(self, mut message_handler: F) -> Self
	where
		F: 'static + FnMut(&WorkletRef, MessageEvent),
	{
		let id_handle = Rc::clone(&self.id);
		let message_handler_handle = Rc::downgrade(&self.message_handler);
		RefCell::borrow_mut(&self.message_handler).replace(MessageHandler::function({
			let mut handle = None;
			move |event: web_sys::MessageEvent| {
				let handle = handle.get_or_insert_with(|| {
					let worklet: AudioWorkletNode = event.target().unwrap().unchecked_into();
					let port = worklet.port().unwrap();
					WorkletRef::new(
						worklet,
						Rc::clone(&id_handle),
						port,
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
					let worklet: AudioWorkletNode = event.target().unwrap().unchecked_into();
					let port = worklet.port().unwrap();
					WorkletRef::new(
						worklet,
						Rc::clone(&id_handle),
						port,
						Weak::clone(&message_handler_handle),
					)
				});
				message_handler(handle, MessageEvent::new(event))
			}
		}));
		self
	}

	#[cfg(feature = "message")]
	pub fn worklet_message_handler<F>(mut self, mut message_handler: F) -> Self
	where
		F: 'static + FnMut(&WorkletContext, MessageEvent) + Send,
	{
		self.worklet_message_handler = Some(SendMessageHandler::function(|context| {
			move |event: web_sys::MessageEvent| {
				message_handler(&context, MessageEvent::new(event));
			}
		}));
		self
	}

	#[cfg(feature = "message")]
	pub fn worklet_message_handler_async<F1, F2>(mut self, mut message_handler: F1) -> Self
	where
		F1: 'static + FnMut(&WorkletContext, MessageEvent) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		self.worklet_message_handler = Some(SendMessageHandler::future(|context| {
			move |event: web_sys::MessageEvent| message_handler(&context, MessageEvent::new(event))
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
		self.add_internal(context, Task::Function(Box::new(f)))
	}

	pub fn add_async<F1, F2>(
		self,
		context: &BaseAudioContext,
		f: F1,
	) -> Result<WorkletFuture<'_>, WorkletInitError>
	where
		F1: 'static + FnOnce(WorkletContext) -> F2 + Send,
		F2: 'static + Future<Output = ()>,
	{
		let task = Task::Future(Box::new(move |context| {
			Box::pin(async move { f(context).await })
		}));

		self.add_internal(context, task)
	}

	fn add_internal(
		self,
		context: &BaseAudioContext,
		task: Task,
	) -> Result<WorkletFuture<'_>, WorkletInitError> {
		let init = Reflect::get(context, &"__wasm_worker_init".into()).unwrap();

		if let Some(init) = init.as_bool() {
			debug_assert!(init);

			return Err(WorkletInitError);
		}

		debug_assert!(init.is_undefined());
		debug_assert!(Reflect::set(context, &"__wasm_worker_init".into(), &true.into()).unwrap());

		Ok(WorkletFuture::new(
			Cow::Borrowed(context),
			self.url,
			task,
			self.id,
			#[cfg(feature = "message")]
			self.message_handler,
			#[cfg(feature = "message")]
			self.worklet_message_handler,
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
	message_handler: Option<SendMessageHandler<WorkletContext>>,
}

#[allow(clippy::type_complexity)]
enum Task {
	Function(Box<dyn 'static + FnOnce(WorkletContext) + Send>),
	Future(
		Box<
			dyn 'static
				+ FnOnce(WorkletContext) -> Pin<Box<dyn 'static + Future<Output = ()>>>
				+ Send,
		>,
	),
}

#[doc(hidden)]
#[wasm_bindgen]
#[allow(unreachable_pub)]
pub unsafe fn __wasm_worker_worklet_entry(this: AudioWorkletProcessor, data: *mut Data) {
	let global = js_sys::global().unchecked_into();

	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worklet_entry` from `worklet.js`. The data sent to it should
	// only come from `WorkletFuture::poll()`.
	let data = *unsafe { Box::from_raw(data) };

	let context = WorkletContext::init(
		global,
		this,
		data.id,
		#[cfg(feature = "message")]
		data.message_handler,
	);

	match data.task {
		Task::Function(f) => {
			f(context);
		}
		Task::Future(future) => wasm_bindgen_futures::spawn_local(future(context)),
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WorkletInitError;

impl Display for WorkletInitError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "already added a Wasm module to this worklet")
	}
}

impl Error for WorkletInitError {}
