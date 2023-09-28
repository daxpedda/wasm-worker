use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::task::{ready, Context, Poll};

#[cfg(feature = "futures")]
use futures_core::FusedFuture;
use js_sys::Array;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};
#[cfg(feature = "message")]
use {
	super::super::WorkletContext,
	crate::message::{MessageHandler, SendMessageHandler},
	std::cell::RefCell,
	std::ops::Deref,
};

use super::super::{Worklet, WORKLET_URL};
use super::{Data, Task};
use crate::common::ID_COUNTER;

#[derive(Debug)]
#[must_use = "does nothing if not polled"]
pub struct WorkletFuture<'context>(Option<Inner<'context>>);

struct Inner<'context> {
	context: Cow<'context, BaseAudioContext>,
	task: Task,
	id: Rc<Cell<Result<u64, u64>>>,
	#[cfg(feature = "message")]
	message_handler: Rc<RefCell<Option<MessageHandler>>>,
	#[cfg(feature = "message")]
	worklet_message_handler: Option<SendMessageHandler<WorkletContext>>,
	promise: JsFuture,
}

impl<'context> WorkletFuture<'context> {
	pub(super) fn new(
		context: Cow<'context, BaseAudioContext>,
		task: Task,
		id: Rc<Cell<Result<u64, u64>>>,
		#[cfg(feature = "message")] message_handler: Rc<RefCell<Option<MessageHandler>>>,
		#[cfg(feature = "message")] worklet_message_handler: Option<
			SendMessageHandler<WorkletContext>,
		>,
	) -> Self {
		let promise = JsFuture::from(WORKLET_URL.with(|url| {
			context
				.audio_worklet()
				.unwrap()
				.add_module(url.as_raw())
				.unwrap()
		}));

		Self(Some(Inner {
			context,
			task,
			id,
			#[cfg(feature = "message")]
			message_handler,
			#[cfg(feature = "message")]
			worklet_message_handler,
			promise,
		}))
	}

	pub fn into_static(self) -> WorkletFuture<'static> {
		WorkletFuture(self.0.map(
			|Inner {
			     context,
			     task,
			     id,
			     #[cfg(feature = "message")]
			     message_handler,
			     #[cfg(feature = "message")]
			     worklet_message_handler,
			     promise: state,
			 }| Inner {
				context: Cow::Owned(context.into_owned()),
				task,
				id,
				#[cfg(feature = "message")]
				message_handler,
				#[cfg(feature = "message")]
				worklet_message_handler,
				promise: state,
			},
		))
	}
}

impl Future for WorkletFuture<'_> {
	type Output = Worklet;

	#[track_caller]
	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let inner = self.0.as_mut().expect("polled after `Ready`");

		let result = ready!(Pin::new(&mut inner.promise).poll(cx));
		let Inner {
			context,
			task,
			id,
			#[cfg(feature = "message")]
			message_handler,
			#[cfg(feature = "message")]
			worklet_message_handler,
			..
		} = self.0.take().unwrap();

		let result = result.unwrap();
		debug_assert!(
			result.is_undefined(),
			"expected `Worklet.addModule()` to return `undefined`"
		);

		let new_id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
		id.set(Ok(new_id));
		let data = Box::into_raw(Box::new(Data {
			id: new_id,
			task,
			#[cfg(feature = "message")]
			message_handler: worklet_message_handler,
		}));

		let mut options = AudioWorkletNodeOptions::new();
		options.processor_options(Some(&Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&data.into(),
		)));

		let node = AudioWorkletNode::new_with_options(&context, "__wasm_worker_InitWasm", &options)
			.unwrap();

		#[cfg(feature = "message")]
		let port = node.port().unwrap();

		#[cfg(feature = "message")]
		if let Some(message_handler) = RefCell::borrow(&message_handler).deref() {
			port.set_onmessage(Some(message_handler));
		}

		Poll::Ready(Worklet::new(
			node,
			id,
			#[cfg(feature = "message")]
			port,
			#[cfg(feature = "message")]
			message_handler,
		))
	}
}

#[cfg(feature = "futures")]
impl FusedFuture for WorkletFuture<'_> {
	fn is_terminated(&self) -> bool {
		self.0.is_none()
	}
}

impl Debug for Inner<'_> {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Inner")
			.field("context", &self.context)
			.field("f", &"Box<FnOnce>")
			.field("state", &self.promise)
			.finish()
	}
}
