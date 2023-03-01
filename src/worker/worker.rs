use std::cell::Cell;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::rc::Rc;

#[cfg(feature = "message")]
use {
	crate::message::{Message, MessageEvent, MessageHandler, SendMessages, TransferError},
	std::borrow::Cow,
	std::cell::RefCell,
	std::future::Future,
	std::rc::Weak,
};

use crate::common::{Tls, EXPORTS};

#[derive(Clone, Debug)]
pub struct Worker {
	worker: web_sys::Worker,
	id: Rc<Cell<Option<usize>>>,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
}

impl Drop for Worker {
	fn drop(&mut self) {
		#[cfg(feature = "message")]
		if Rc::strong_count(&self.message_handler) == 1 {
			self.worker.set_onmessage(None);
		}
	}
}

impl Eq for Worker {}

impl PartialEq for Worker {
	fn eq(&self, other: &Self) -> bool {
		self.worker == other.worker
	}
}

impl WorkerOrRef for Worker {
	#[cfg(feature = "message")]
	fn handle_ref(&self) -> WorkerRef {
		WorkerRef {
			worker: self.worker.clone(),
			id: Rc::clone(&self.id),
			#[cfg(feature = "message")]
			message_handler: Rc::downgrade(&self.message_handler),
		}
	}

	fn worker(&self) -> &web_sys::Worker {
		&self.worker
	}

	fn id(&self) -> &Rc<Cell<Option<usize>>> {
		&self.id
	}

	#[cfg(feature = "message")]
	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<MessageHandler>>>>> {
		Some(Cow::Borrowed(&self.message_handler))
	}
}

impl Worker {
	pub(super) fn new(
		worker: web_sys::Worker,
		id: Rc<Cell<Option<usize>>>,
		#[cfg(feature = "message")] message_handler: Rc<RefCell<Option<MessageHandler>>>,
	) -> Self {
		Self {
			worker,
			id,
			#[cfg(feature = "message")]
			message_handler,
		}
	}

	#[must_use]
	pub const fn as_raw(&self) -> &web_sys::Worker {
		&self.worker
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkerOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn clear_message_handler(&self) {
		<Self as WorkerOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler<F>(&self, new_message_handler: F)
	where
		F: 'static + FnMut(&WorkerRef, MessageEvent),
	{
		<Self as WorkerOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler_async<F1, F2>(&self, new_message_handler: F1)
	where
		F1: 'static + FnMut(&WorkerRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		<Self as WorkerOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn transfer_messages<M, I>(&self, messages: M) -> Result<(), TransferError>
	where
		M: IntoIterator<Item = I>,
		I: Into<Message>,
	{
		<Self as WorkerOrRef>::transfer_messages(self, messages)
	}

	#[allow(clippy::same_name_method)]
	pub fn terminate(&self) {
		<Self as WorkerOrRef>::terminate(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		<Self as WorkerOrRef>::destroy(self, tls)
	}
}

#[derive(Clone, Debug)]
#[cfg(feature = "message")]
pub struct WorkerRef {
	worker: web_sys::Worker,
	id: Rc<Cell<Option<usize>>>,
	#[cfg(feature = "message")]
	message_handler: Weak<RefCell<Option<MessageHandler>>>,
}

#[cfg(feature = "message")]
impl WorkerOrRef for WorkerRef {
	fn handle_ref(&self) -> WorkerRef {
		self.clone()
	}

	fn worker(&self) -> &web_sys::Worker {
		&self.worker
	}

	fn id(&self) -> &Rc<Cell<Option<usize>>> {
		&self.id
	}

	#[cfg(feature = "message")]
	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<MessageHandler>>>>> {
		Weak::upgrade(&self.message_handler).map(Cow::Owned)
	}
}

#[cfg(feature = "message")]
impl WorkerRef {
	pub(super) fn new(
		worker: web_sys::Worker,
		id: Rc<Cell<Option<usize>>>,
		#[cfg(feature = "message")] message_handler: Weak<RefCell<Option<MessageHandler>>>,
	) -> Self {
		Self {
			worker,
			id,
			#[cfg(feature = "message")]
			message_handler,
		}
	}

	#[must_use]
	pub const fn as_raw(&self) -> &web_sys::Worker {
		&self.worker
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkerOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn clear_message_handler(&self) {
		<Self as WorkerOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler<F>(&self, new_message_handler: F)
	where
		F: 'static + FnMut(&Self, MessageEvent),
	{
		<Self as WorkerOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler_async<F1, F2>(&self, new_message_handler: F1)
	where
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		<Self as WorkerOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn transfer_messages<M, I>(&self, messages: M) -> Result<(), TransferError>
	where
		M: IntoIterator<Item = I>,
		I: Into<Message>,
	{
		<Self as WorkerOrRef>::transfer_messages(self, messages)
	}

	#[allow(clippy::same_name_method)]
	pub fn terminate(&self) {
		<Self as WorkerOrRef>::terminate(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		<Self as WorkerOrRef>::destroy(self, tls)
	}
}

trait WorkerOrRef: Debug + Sized {
	#[cfg(feature = "message")]
	fn handle_ref(&self) -> WorkerRef;

	fn worker(&self) -> &web_sys::Worker;

	fn id(&self) -> &Rc<Cell<Option<usize>>>;

	#[cfg(feature = "message")]
	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<MessageHandler>>>>>;

	#[cfg(feature = "message")]
	fn has_message_handler(&self) -> bool {
		self.message_handler().map_or(false, |message_handler| {
			RefCell::borrow(&message_handler).is_some()
		})
	}

	#[cfg(feature = "message")]
	fn clear_message_handler(&self) {
		if let Some(message_handler) = self.message_handler() {
			message_handler.take();
			self.worker().set_onmessage(None);
		}
	}

	#[cfg(feature = "message")]
	fn set_message_handler<F: 'static + FnMut(&WorkerRef, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) {
		if let Some(message_handler) = self.message_handler() {
			let handle = self.handle_ref();

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let message_handler = message_handler.insert(MessageHandler::classic(move |event| {
				new_message_handler(&handle, MessageEvent::new(event));
			}));

			self.worker().set_onmessage(Some(message_handler));
		}
	}

	#[cfg(feature = "message")]
	fn set_message_handler_async<
		F1: 'static + FnMut(&WorkerRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		mut new_message_handler: F1,
	) {
		if let Some(message_handler) = self.message_handler() {
			let handle = self.handle_ref();

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let message_handler = message_handler.insert(MessageHandler::future(move |event| {
				new_message_handler(&handle, MessageEvent::new(event))
			}));

			self.worker().set_onmessage(Some(message_handler));
		}
	}

	#[cfg(feature = "message")]
	fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		&self,
		messages: M,
	) -> Result<(), TransferError> {
		self.worker().transfer_messages(messages)
	}

	fn terminate(&self) {
		self.worker().terminate();
	}

	fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		if let Some(id) = self.id().get() {
			if id == tls.id {
				self.id().take();
				self.terminate();

				EXPORTS.with(|exports| {
					// SAFETY: The id is uniquely created in `WorkerBuilder::spawn_internal()`
					// through an `AtomicUsize` counter. It then is saved here and sent to the
					// worker and used in generating `Tls`. The ids are then compared above and if
					// they match, the state is change to `None` preventing any subsequent calls.
					unsafe { exports.thread_destroy(&tls.tls_base(), &tls.stack_alloc()) };
				});

				Ok(())
			} else {
				Err(DestroyError::Match { handle: self, tls })
			}
		} else {
			Err(DestroyError::Already(tls))
		}
	}
}

#[derive(Debug)]
pub enum DestroyError<T>
where
	T: Debug,
{
	Already(Tls),
	Match { handle: T, tls: Tls },
}

impl<T> Display for DestroyError<T>
where
	T: Debug,
{
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Already(_) => write!(f, "this worker was already destroyed"),
			Self::Match { .. } => {
				write!(f, "`Tls` value given does not belong to this worker")
			}
		}
	}
}

impl<T> Error for DestroyError<T> where T: Debug {}
