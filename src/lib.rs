#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! - Registry:
//!   - Termination from Handle
//!   - List all workers and terminate
//!   - Get raw `Worker`
//! - Messaging:
//!   - from local only Handle
//!   - from handle through Registry
//! - Nested workers
//! - Worklets

mod flag;
#[cfg(feature = "registry")]
mod registry;
mod try_catch;

use std::error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::PanicInfo;
use std::pin::Pin;
use std::task::{Context, Poll};

use flag::Flag;
use futures_channel::mpsc::{self, UnboundedSender};
use futures_channel::oneshot::{self, Canceled, Receiver};
use futures_util::future::FusedFuture;
use futures_util::{FutureExt, StreamExt};
use js_sys::Array;
use once_cell::unsync::OnceCell;
use try_catch::TryFuture;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{console, Blob, BlobPropertyBag, DedicatedWorkerGlobalScope, Url, Worker};

thread_local! {
	static STATE: StateWrapper = StateWrapper(OnceCell::new());
}

struct StateWrapper(OnceCell<State>);

impl Deref for StateWrapper {
	type Target = State;

	fn deref(&self) -> &Self::Target {
		self.0.get_or_init(|| State::Root(RootState::new()))
	}
}

impl StateWrapper {
	fn init_worker(&self, message_sender: UnboundedSender<Message>, terminate_flag: Flag) {
		self.0
			.set(State::Worker(WorkerState::new(
				message_sender,
				terminate_flag,
			)))
			.expect("`State` was already initialized");
	}
}

#[derive(Debug)]
enum State {
	Root(RootState),
	Worker(WorkerState),
}

impl State {
	fn root(&self) -> &RootState {
		match self {
			Self::Root(state) => state,
			Self::Worker(_) => panic!("expected root context"),
		}
	}
}

#[derive(Debug)]
struct RootState {
	worker_url: String,
	message_sender: UnboundedSender<Message>,
}

impl RootState {
	fn new() -> Self {
		let script = format!(
			"importScripts('{}');\n{}",
			wasm_bindgen::script_url(),
			include_str!("worker/script.js")
		);

		let sequence = Array::of1(&JsValue::from(script));
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property);

		let worker_url = blob
			.and_then(|blob| Url::create_object_url_with_blob(&blob))
			.expect("worker `Url` could not be created");

		let (message_sender, mut message_receiver) = mpsc::unbounded();

		wasm_bindgen_futures::spawn_local(async move {
			while let Some(message) = message_receiver.next().await {
				match message {
					Message::Spawn {
						terminate_flag,
						worker_builder,
						work,
					} => STATE.with(|state| {
						spawn_from_root(state.root(), terminate_flag, worker_builder, work);
					}),
				}
			}
		});

		Self {
			worker_url,
			message_sender,
		}
	}
}

impl Drop for RootState {
	fn drop(&mut self) {
		if let Err(error) = Url::revoke_object_url(&self.worker_url) {
			console::warn_1(&format!("worker `Url` could not be deallocated: {error:?}").into());
		}
	}
}

#[derive(Debug)]
struct WorkerState {
	message_sender: UnboundedSender<Message>,
	terminate_flag: Flag,
}

impl WorkerState {
	fn new(message_sender: UnboundedSender<Message>, terminate_flag: Flag) -> Self {
		{
			let mut terminate_flag = terminate_flag.clone();
			wasm_bindgen_futures::spawn_local(async move {
				assert_eq!(
					(&mut terminate_flag).await,
					flag::State::Raised,
					"worker was already terminated"
				);
				terminate_flag.complete();
				__wasm_worker_close();
				unreachable!("`__wasm_worker_close()` did not close the worker");
			});
		}

		Self {
			message_sender,
			terminate_flag,
		}
	}
}

#[must_use]
pub fn is_root() -> bool {
	STATE.with(|state| match &**state {
		State::Root(_) => true,
		State::Worker(_) => false,
	})
}

#[must_use]
pub fn is_worker() -> bool {
	STATE.with(|state| match &**state {
		State::Worker(_) => true,
		State::Root(_) => false,
	})
}

#[derive(Debug)]
pub struct WorkerHandle<R> {
	return_receiver: Receiver<Result<R, String>>,
	terminate_flag: Flag,
	gracefully_finished: GracefullyFinished,
}

#[derive(Debug)]
pub struct WorkerLocalHandle<R> {
	handle: WorkerHandle<R>,
	worker: Worker,
}

#[derive(Debug)]
enum GracefullyFinished {
	Unknown(WorkerBuilder),
	Known(bool),
}

impl<R> Future for WorkerHandle<R> {
	type Output = Result<R, Error>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		macro_rules! poll_return {
			() => {
				if let Poll::Ready(result) = Pin::new(&mut self.return_receiver).poll(cx) {
					let result = result.map_err(Error::from_canceled)?.map_err(Error::Error);
					return Poll::Ready(result);
				}
			};
		}

		poll_return!();

		if let Poll::Ready(flag::State::Completed) = Pin::new(&mut self.terminate_flag).poll(cx) {
			poll_return!();

			return Poll::Ready(Err(Error::Terminated));
		}

		Poll::Pending
	}
}

impl<R> Future for WorkerLocalHandle<R> {
	type Output = Result<R, Error>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.handle.poll_unpin(cx)
	}
}

impl<R> FusedFuture for WorkerHandle<R> {
	fn is_terminated(&self) -> bool {
		self.return_receiver.is_terminated() || self.terminate_flag.is_completed()
	}
}

impl<R> FusedFuture for WorkerLocalHandle<R> {
	fn is_terminated(&self) -> bool {
		FusedFuture::is_terminated(&self.handle)
	}
}

impl<R> WorkerHandle<R> {
	fn try_recv(&mut self) {
		
	}

	#[allow(clippy::same_name_method)]
	#[must_use]
	pub fn is_terminated(&self) -> bool {
		self.terminate_flag.is_completed()
	}

	pub fn request_termination(&mut self) -> Result<Option<R>, Error> {
		self.terminate_flag.raise();

		self.try_join()
	}

	pub fn try_join(&mut self) -> Result<Option<R>, Error> {
		macro_rules! try_recv_return {
			() => {
				if let Some(result) = self
					.return_receiver
					.try_recv()
					.map_err(Error::from_canceled)?
				{
					return result.map(Some).map_err(Error::Error);
				}
			};
		}

		try_recv_return!();

		if self.terminate_flag.is_completed() {
			try_recv_return!();

			return Err(Error::Terminated);
		}

		Ok(None)
	}

	#[cfg(feature = "blocking")]
	pub fn join(self) -> Result<R, Error> {
		use pollster::FutureExt;

		self.block_on()
	}
}

impl<R> WorkerLocalHandle<R> {
	#[allow(clippy::same_name_method)]
	#[must_use]
	pub fn is_terminated(&self) -> bool {
		self.handle.is_terminated()
	}

	pub fn request_termination(&mut self) -> Result<Option<R>, Error> {
		self.handle.request_termination()
	}

	pub fn terminate(mut self) -> Result<R, Error> {
		self.worker.terminate();

		if let Some(result) = self
			.handle
			.return_receiver
			.try_recv()
			.map_err(Error::from_canceled)?
		{
			result.map_err(Error::Error)
		} else {
			Err(Error::Terminated)
		}
	}

	pub fn try_join(&mut self) -> Result<Option<R>, Error> {
		self.handle.try_join()
	}

	#[cfg(feature = "blocking")]
	pub fn join(self) -> Result<R, Error> {
		self.handle.join()
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn to_send_handle(self) -> WorkerHandle<R> {
		self.handle
	}

	#[cfg(feature = "raw")]
	#[must_use]
	pub const fn raw_worker(&self) -> &Worker {
		&self.worker
	}
}

#[derive(Clone, Copy, Debug)]
#[must_use = "does nothing if not spawned"]
pub struct WorkerBuilder {
	pub close_on_finish: bool,
	pub close_on_panic: bool,
}

#[derive(Clone, Copy, Debug)]
#[must_use = "does nothing if not spawned"]
pub struct LocalWorkerBuilder {
	pub builder: WorkerBuilder,
	phantom: PhantomData<Worker>,
}

impl Default for WorkerBuilder {
	fn default() -> Self {
		Self::new()
	}
}

impl WorkerBuilder {
	pub const fn new() -> Self {
		Self {
			close_on_finish: true,
			close_on_panic: false,
		}
	}

	pub const fn close_on_finish(mut self, close_on_finish: bool) -> Self {
		self.close_on_finish = close_on_finish;
		self
	}

	pub const fn close_on_panic(mut self, close_on_panic: bool) -> Self {
		self.close_on_panic = close_on_panic;
		self
	}

	pub fn spawn<F, R>(self, f: F) -> WorkerHandle<R>
	where
		F: 'static + FnOnce() -> R + Send,
		R: 'static + Send,
	{
		let (work, handle) = spawn_sync_internal(f, self);

		spawn_internal(handle.terminate_flag.clone(), self, work);

		handle
	}

	pub fn spawn_async<F1, F2, R>(self, f: F1) -> WorkerHandle<R>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = R>,
		R: 'static + Send,
	{
		let (work, handle) = spawn_async_internal(f, self);

		spawn_internal(handle.terminate_flag.clone(), self, work);

		handle
	}

	pub fn to_local_builder(self) -> Result<LocalWorkerBuilder, Self> {
		is_root()
			.then_some(LocalWorkerBuilder {
				builder: self,
				phantom: PhantomData,
			})
			.ok_or(self)
	}
}

impl LocalWorkerBuilder {
	#[must_use]
	pub fn new() -> Option<Self> {
		is_root().then_some(Self {
			builder: WorkerBuilder::default(),
			phantom: PhantomData,
		})
	}

	pub const fn close_on_finish(mut self, close_on_finish: bool) -> Self {
		self.builder.close_on_finish = close_on_finish;
		self
	}

	pub const fn close_on_panic(mut self, close_on_panic: bool) -> Self {
		self.builder.close_on_panic = close_on_panic;
		self
	}

	pub fn spawn<F, R>(self, f: F) -> WorkerLocalHandle<R>
	where
		F: 'static + FnOnce() -> R + Send,
		R: 'static + Send,
	{
		let (work, handle) = spawn_sync_internal(f, self.builder);

		let worker = STATE.with(|state| {
			spawn_from_root(
				state.root(),
				handle.terminate_flag.clone(),
				self.builder,
				work,
			)
		});

		WorkerLocalHandle { handle, worker }
	}

	pub fn spawn_async<F1, F2, R>(self, f: F1) -> WorkerLocalHandle<R>
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = R>,
		R: 'static + Send,
	{
		let (work, handle) = spawn_async_internal(f, self.builder);

		let worker = STATE.with(|state| {
			spawn_from_root(
				state.root(),
				handle.terminate_flag.clone(),
				self.builder,
				work,
			)
		});

		WorkerLocalHandle { handle, worker }
	}

	pub const fn to_send_builder(self) -> WorkerBuilder {
		self.builder
	}
}

pub fn spawn<F, R>(f: F) -> WorkerHandle<R>
where
	F: 'static + FnOnce() -> R + Send,
	R: 'static + Send,
{
	WorkerBuilder::default().spawn(f)
}

pub fn spawn_async<F1, F2, R>(f: F1) -> WorkerHandle<R>
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = R>,
	R: 'static + Send,
{
	WorkerBuilder::default().spawn_async(f)
}

fn spawn_sync_internal<F, R>(f: F, builder: WorkerBuilder) -> (Work, WorkerHandle<R>)
where
	F: 'static + FnOnce() -> R + Send,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();

	let work = Work::Closure(Box::new(move || {
		let result = try_catch::try_(f);
		let success = result.is_ok();
		let _canceled = sender.send(result);

		success
	}));

	let terminate_flag = Flag::new();

	let gracefully_finished = GracefullyFinished::Unknown(builder);

	let handle = WorkerHandle {
		return_receiver: receiver,
		terminate_flag,
		gracefully_finished,
	};

	(work, handle)
}

fn spawn_async_internal<F1, F2, R>(f: F1, builder: WorkerBuilder) -> (Work, WorkerHandle<R>)
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = R>,
	R: 'static + Send,
{
	let (sender, receiver) = oneshot::channel();

	let work = Work::Future(Box::new(move || {
		Box::pin(async move {
			let result = match try_catch::try_(f) {
				Ok(f) => TryFuture::wrap(f).await,
				Err(error) => Err(error),
			};
			let success = result.is_ok();
			let _canceled = sender.send(result);

			success
		})
	}));

	let terminate_flag = Flag::new();

	let gracefully_finished = GracefullyFinished::Unknown(builder);

	let handle = WorkerHandle {
		return_receiver: receiver,
		terminate_flag,
		gracefully_finished,
	};

	(work, handle)
}

fn spawn_internal(terminate_flag: Flag, worker_builder: WorkerBuilder, work: Work) {
	STATE.with(|state| match &**state {
		State::Root(state) => {
			spawn_from_root(state, terminate_flag, worker_builder, work);
		}
		State::Worker(state) => spawn_from_worker(state, terminate_flag, worker_builder, work),
	});
}

fn spawn_from_root(
	root_state: &RootState,
	terminate_flag: Flag,
	worker_builder: WorkerBuilder,
	work: Work,
) -> Worker {
	let task = Task {
		terminate_flag,
		message_sender: root_state.message_sender.clone(),
		worker_builder,
		work,
	};
	let task = Box::into_raw(Box::new(task));

	let worker = Worker::new(&root_state.worker_url).expect("`Worker.new()` failed");
	let init = Array::of3(
		&wasm_bindgen::module(),
		&wasm_bindgen::memory(),
		&task.into(),
	);

	worker
		.post_message(&init)
		.expect("`Worker.postMessage()` failed");

	worker
}

fn spawn_from_worker(
	state: &WorkerState,
	terminate_flag: Flag,
	worker_builder: WorkerBuilder,
	work: Work,
) {
	state
		.message_sender
		.unbounded_send(Message::Spawn {
			terminate_flag,
			worker_builder,
			work,
		})
		.expect("receiver dropped somehow");
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Task {
	terminate_flag: Flag,
	message_sender: UnboundedSender<Message>,
	worker_builder: WorkerBuilder,
	work: Work,
}

enum Work {
	Closure(Box<dyn 'static + FnOnce() -> bool + Send>),
	Future(Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = bool>>> + Send>),
}

impl Debug for Work {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Closure(_) => f.debug_struct("Closure").finish(),
			Self::Future(_) => f.debug_struct("Future").finish(),
		}
	}
}

#[doc(hidden)]
#[allow(clippy::future_not_send)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(task: *mut Task) -> bool {
	js_sys::global()
		.unchecked_into::<DedicatedWorkerGlobalScope>()
		.set_onmessage(None);

	let Task {
		terminate_flag,
		message_sender,
		worker_builder,
		work,
	} =
	// SAFETY: The argument is an address that has to be a valid pointer to a `Task`.
	*unsafe { Box::from_raw(task) };

	STATE.with(|state| StateWrapper::init_worker(state, message_sender, terminate_flag));

	let success = match work {
		Work::Closure(fn_) => fn_(),
		Work::Future(fn_) => fn_().await,
	};

	if worker_builder.close_on_finish {
		true
	} else {
		!success && worker_builder.close_on_panic
	}
}

#[derive(Debug)]
pub enum Error {
	Error(String),
	Terminated,
	Taken,
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Error(error) => write!(f, "{error}"),
			Self::Terminated => write!(f, "worker terminated before receiving a return value"),
			Self::Taken => write!(f, "return value was already taken"),
		}
	}
}

impl From<Error> for JsValue {
	fn from(error: Error) -> Self {
		error.to_string().into()
	}
}

impl error::Error for Error {}

impl Error {
	const fn from_canceled(_: Canceled) -> Self {
		Self::Taken
	}
}

pub fn hook(panic_info: &PanicInfo<'_>) -> ! {
	wasm_bindgen::throw_str(&panic_info.to_string());
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();
}

pub fn terminate() -> ! {
	STATE.with(|state| {
		if let State::Worker(state) = &**state {
			state.terminate_flag.complete();
			__wasm_worker_close();
			unreachable!("`__wasm_worker_close()` did not close the worker");
		} else {
			panic!("called `terminate()` from the root context");
		}
	})
}

#[derive(Debug)]
enum Message {
	Spawn {
		terminate_flag: Flag,
		worker_builder: WorkerBuilder,
		work: Work,
	},
}
