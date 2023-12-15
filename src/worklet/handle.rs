use std::cell::Cell;
use std::fmt::Debug;
use std::rc::Rc;

use web_sys::AudioWorkletNode;
#[cfg(feature = "message")]
use {
	crate::message::{Message, MessageEvent, MessageHandler, SendMessages, TransferError},
	std::borrow::Cow,
	std::cell::RefCell,
	std::future::Future,
	std::rc::Weak,
	web_sys::MessagePort,
};

use crate::common::{DestroyError, Exports, Tls};

#[derive(Clone, Debug)]
pub struct Worklet {
	#[allow(clippy::struct_field_names)]
	worklet: AudioWorkletNode,
	id: Rc<Cell<Result<u64, u64>>>,
	#[cfg(feature = "message")]
	port: MessagePort,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
}

#[cfg_attr(not(feature = "message"), allow(clippy::empty_drop))]
impl Drop for Worklet {
	fn drop(&mut self) {
		#[cfg(feature = "message")]
		if Rc::strong_count(&self.message_handler) == 1 {
			self.port.set_onmessage(None);
		}
	}
}

impl Eq for Worklet {}

impl PartialEq for Worklet {
	fn eq(&self, other: &Self) -> bool {
		self.worklet == other.worklet
	}
}

impl WorkletOrRef for Worklet {
	#[cfg(feature = "message")]
	fn handle_ref(&self) -> WorkletRef {
		WorkletRef {
			worklet: self.worklet.clone(),
			id: Rc::clone(&self.id),
			port: self.port.clone(),
			message_handler: Rc::downgrade(&self.message_handler),
		}
	}

	fn worklet(&self) -> &AudioWorkletNode {
		&self.worklet
	}

	fn id(&self) -> &Rc<Cell<Result<u64, u64>>> {
		&self.id
	}

	#[cfg(feature = "message")]
	fn port(&self) -> &MessagePort {
		&self.port
	}

	#[cfg(feature = "message")]
	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<MessageHandler>>>>> {
		Some(Cow::Borrowed(&self.message_handler))
	}
}

impl Worklet {
	pub(super) fn new(
		worklet: AudioWorkletNode,
		id: Rc<Cell<Result<u64, u64>>>,
		#[cfg(feature = "message")] port: MessagePort,
		#[cfg(feature = "message")] message_handler: Rc<RefCell<Option<MessageHandler>>>,
	) -> Self {
		Self {
			worklet,
			id,
			#[cfg(feature = "message")]
			port,
			#[cfg(feature = "message")]
			message_handler,
		}
	}

	#[must_use]
	pub const fn as_raw(&self) -> &AudioWorkletNode {
		&self.worklet
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkletOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn clear_message_handler(&self) {
		<Self as WorkletOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler<F>(&self, new_message_handler: F)
	where
		F: 'static + FnMut(&WorkletRef, MessageEvent),
	{
		<Self as WorkletOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn set_message_handler_async<F1, F2>(&self, new_message_handler: F1)
	where
		F1: 'static + FnMut(&WorkletRef, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		<Self as WorkletOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	#[cfg(feature = "message")]
	pub fn transfer_messages<I, M>(&self, messages: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = M>,
		M: Into<Message>,
	{
		<Self as WorkletOrRef>::transfer_messages(self, messages)
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	pub fn id(&self) -> u64 {
		let (Ok(id) | Err(id)) = self.id.get();
		id
	}

	#[must_use]
	pub fn destroyed(&self) -> bool {
		self.id.get().is_ok()
	}

	/// # Safety
	///
	/// Must only be called if the worklet has finished running.
	#[allow(clippy::same_name_method)]
	pub unsafe fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		// SAFETY: See documentation in calling function.
		unsafe { <Self as WorkletOrRef>::destroy(self, tls) }
	}
}

#[derive(Clone, Debug)]
#[cfg(feature = "message")]
pub struct WorkletRef {
	worklet: AudioWorkletNode,
	id: Rc<Cell<Result<u64, u64>>>,
	port: MessagePort,
	message_handler: Weak<RefCell<Option<MessageHandler>>>,
}

#[cfg(feature = "message")]
impl WorkletOrRef for WorkletRef {
	fn handle_ref(&self) -> WorkletRef {
		self.clone()
	}

	fn worklet(&self) -> &AudioWorkletNode {
		&self.worklet
	}

	fn id(&self) -> &Rc<Cell<Result<u64, u64>>> {
		&self.id
	}

	fn port(&self) -> &MessagePort {
		&self.port
	}

	fn message_handler(&self) -> Option<Cow<'_, Rc<RefCell<Option<MessageHandler>>>>> {
		Weak::upgrade(&self.message_handler).map(Cow::Owned)
	}
}

#[cfg(feature = "message")]
impl WorkletRef {
	pub(super) fn new(
		worklet: AudioWorkletNode,
		id: Rc<Cell<Result<u64, u64>>>,
		port: MessagePort,
		message_handler: Weak<RefCell<Option<MessageHandler>>>,
	) -> Self {
		Self {
			worklet,
			id,
			port,
			message_handler,
		}
	}

	#[must_use]
	pub const fn as_raw(&self) -> &AudioWorkletNode {
		&self.worklet
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	pub fn has_message_handler(&self) -> bool {
		<Self as WorkletOrRef>::has_message_handler(self)
	}

	#[allow(clippy::same_name_method)]
	pub fn clear_message_handler(&self) {
		<Self as WorkletOrRef>::clear_message_handler(self);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler<F>(&self, new_message_handler: F)
	where
		F: 'static + FnMut(&Self, MessageEvent),
	{
		<Self as WorkletOrRef>::set_message_handler(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn set_message_handler_async<F1, F2>(&self, new_message_handler: F1)
	where
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		<Self as WorkletOrRef>::set_message_handler_async(self, new_message_handler);
	}

	#[allow(clippy::same_name_method)]
	pub fn transfer_messages<I, M>(&self, messages: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = M>,
		M: Into<Message>,
	{
		<Self as WorkletOrRef>::transfer_messages(self, messages)
	}

	#[must_use]
	#[allow(clippy::same_name_method)]
	pub fn id(&self) -> u64 {
		let (Ok(id) | Err(id)) = self.id.get();
		id
	}

	#[must_use]
	pub fn destroyed(&self) -> bool {
		self.id.get().is_ok()
	}

	/// # Safety
	///
	/// Must only be called if the worklet has finished running.
	#[allow(clippy::same_name_method)]
	pub unsafe fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		// SAFETY: See documentation in calling function.
		unsafe { <Self as WorkletOrRef>::destroy(self, tls) }
	}
}

trait WorkletOrRef: Debug + Sized {
	#[cfg(feature = "message")]
	fn handle_ref(&self) -> WorkletRef;

	fn worklet(&self) -> &AudioWorkletNode;

	fn id(&self) -> &Rc<Cell<Result<u64, u64>>>;

	#[cfg(feature = "message")]
	fn port(&self) -> &MessagePort;

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
			self.port().set_onmessage(None);
		}
	}

	#[cfg(feature = "message")]
	fn set_message_handler<F: 'static + FnMut(&WorkletRef, MessageEvent)>(
		&self,
		mut new_message_handler: F,
	) {
		if let Some(message_handler) = self.message_handler() {
			let handle = self.handle_ref();

			let mut message_handler = RefCell::borrow_mut(&message_handler);
			let message_handler = message_handler.insert(MessageHandler::function(move |event| {
				new_message_handler(&handle, MessageEvent::new(event));
			}));

			self.port().set_onmessage(Some(message_handler));
		}
	}

	#[cfg(feature = "message")]
	fn set_message_handler_async<
		F1: 'static + FnMut(&WorkletRef, MessageEvent) -> F2,
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

			self.port().set_onmessage(Some(message_handler));
		}
	}

	#[cfg(feature = "message")]
	fn transfer_messages<I: IntoIterator<Item = M>, M: Into<Message>>(
		&self,
		messages: I,
	) -> Result<(), TransferError> {
		self.port().transfer_messages(messages)
	}

	unsafe fn destroy(self, tls: Tls) -> Result<(), DestroyError<Self>> {
		if let Ok(id) = self.id().get() {
			if id == tls.id {
				self.id().set(Err(id));

				Exports::with(|exports| {
					// SAFETY: The id is uniquely created in `WorkletBuilder::spawn_internal()`
					// through an `AtomicUsize` counter. It then is saved here and sent to the
					// worklet and used in generating `Tls`. The ids are then compared above and if
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
