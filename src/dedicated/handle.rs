use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::rc::{Rc, Weak};

use web_sys::Worker;

use super::{Closure, Tls, TransferError, WorkerOrContext, EXPORTS};
use crate::{Message, MessageEvent};

#[derive(Clone, Debug)]
pub struct WorkerHandle {
	worker: Worker,
	id: Rc<Cell<Option<usize>>>,
	message_handler: Rc<RefCell<Option<Closure>>>,
}

impl Drop for WorkerHandle {
	fn drop(&mut self) {
		if Rc::strong_count(&self.message_handler) == 1 {
			self.worker.set_onmessage(None);
		}
	}
}

impl Eq for WorkerHandle {}

impl PartialEq for WorkerHandle {
	fn eq(&self, other: &Self) -> bool {
		self.worker == other.worker
	}
}

impl WorkerHandleOrRef for WorkerHandle {
	fn handle_ref(&self) -> WorkerHandleRef {
		WorkerHandleRef {
			worker: self.worker.clone(),
			id: Rc::clone(&self.id),
			message_handler: Rc::downgrade(&self.message_handler),
		}
	}

	fn worker(&self) -> &Worker {
		&self.worker
	}

	fn id(&self) -> &Rc<Cell<Option<usize>>> {
		&self.id
	}

	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<Closure>>>>> {
		Some(Cow::Borrowed(&self.message_handler))
	}
}

impl WorkerHandle {
	pub(super) fn new(
		worker: Worker,
		id: Rc<Cell<Option<usize>>>,
		message_handler: Rc<RefCell<Option<Closure>>>,
	) -> Self {
		Self {
			worker,
			id,
			message_handler,
		}
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkerHandleOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	pub fn clear_message_handler(&self) {
		<Self as WorkerHandleOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler<F: 'static + FnMut(&WorkerHandleRef, MessageEvent)>(
		&self,
		new_message_handler: F,
	) {
		<Self as WorkerHandleOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler_async<
		F1: 'static + FnMut(&WorkerHandleRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		new_message_handler: F1,
	) {
		<Self as WorkerHandleOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		&self,
		messages: M,
	) -> Result<(), TransferError> {
		<Self as WorkerHandleOrRef>::transfer_messages(self, messages)
	}

	#[allow(clippy::same_name_method)]
	pub fn terminate(&self) {
		<Self as WorkerHandleOrRef>::terminate(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		<Self as WorkerHandleOrRef>::destroy(self, tls)
	}
}

#[derive(Clone, Debug)]
pub struct WorkerHandleRef {
	worker: Worker,
	id: Rc<Cell<Option<usize>>>,
	message_handler: Weak<RefCell<Option<Closure>>>,
}

impl WorkerHandleOrRef for WorkerHandleRef {
	fn handle_ref(&self) -> WorkerHandleRef {
		self.clone()
	}

	fn worker(&self) -> &Worker {
		&self.worker
	}

	fn id(&self) -> &Rc<Cell<Option<usize>>> {
		&self.id
	}

	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<Closure>>>>> {
		Weak::upgrade(&self.message_handler).map(Cow::Owned)
	}
}

impl WorkerHandleRef {
	pub(super) fn new(
		worker: Worker,
		id: Rc<Cell<Option<usize>>>,
		message_handler: Weak<RefCell<Option<Closure>>>,
	) -> Self {
		Self {
			worker,
			id,
			message_handler,
		}
	}

	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.worker
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkerHandleOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	pub fn clear_message_handler(&self) {
		<Self as WorkerHandleOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler<F: 'static + FnMut(&Self, MessageEvent)>(
		&self,
		new_message_handler: F,
	) {
		<Self as WorkerHandleOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler_async<
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		new_message_handler: F1,
	) {
		<Self as WorkerHandleOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		&self,
		messages: M,
	) -> Result<(), TransferError> {
		<Self as WorkerHandleOrRef>::transfer_messages(self, messages)
	}

	#[allow(clippy::same_name_method)]
	pub fn terminate(&self) {
		<Self as WorkerHandleOrRef>::terminate(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		<Self as WorkerHandleOrRef>::destroy(self, tls)
	}
}

trait WorkerHandleOrRef: Debug + Sized {
	fn handle_ref(&self) -> WorkerHandleRef;

	fn worker(&self) -> &Worker;

	fn id(&self) -> &Rc<Cell<Option<usize>>>;

	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<Closure>>>>>;

	fn has_message_handler(&self) -> bool {
		self.message_handler().map_or(false, |message_handler| {
			RefCell::borrow(&message_handler).is_some()
		})
	}

	fn clear_message_handler(&self) {
		if let Some(message_handler) = self.message_handler() {
			message_handler.take();
			self.worker().set_onmessage(None);
		}
	}

	fn set_message_handler<F: 'static + FnMut(&WorkerHandleRef, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) {
		if let Some(message_handler) = self.message_handler() {
			let handle = self.handle_ref();

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let message_handler = message_handler.insert(Closure::classic(move |event| {
				new_message_handler(&handle, MessageEvent::new(event));
			}));

			self.worker().set_onmessage(Some(message_handler));
		}
	}

	fn set_message_handler_async<
		F1: 'static + FnMut(&WorkerHandleRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	>(
		&self,
		mut new_message_handler: F1,
	) {
		if let Some(message_handler) = self.message_handler() {
			let handle = self.handle_ref();

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let message_handler = message_handler.insert(Closure::future(move |event| {
				new_message_handler(&handle, MessageEvent::new(event))
			}));

			self.worker().set_onmessage(Some(message_handler));
		}
	}

	fn transfer_messages<M: IntoIterator<Item = I>, I: Into<Message>>(
		&self,
		messages: M,
	) -> Result<(), TransferError> {
		WorkerOrContext::Worker(self.worker()).transfer_messages(messages)
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
pub enum DestroyError<T: Debug> {
	Already(Tls),
	Match { handle: T, tls: Tls },
}

impl<T: Debug> Display for DestroyError<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Already(_) => write!(f, "this worker was already destroyed"),
			Self::Match { .. } => {
				write!(f, "`Tls` value given does not belong to this worker")
			}
		}
	}
}

impl<T: Debug> Error for DestroyError<T> {}
