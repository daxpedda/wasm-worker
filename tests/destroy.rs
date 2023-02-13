//! Tests destroying a worker.

mod util;

use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;

use anyhow::Result;
use futures_channel::oneshot;
use futures_util::future::{self, Either};
use js_sys::ArrayBuffer;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{DestroyError, Message, WorkerBuilder};

use self::util::{Flag, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerHandle::destroy()`](wasm_worker::WorkerHandle::destroy).
#[wasm_bindgen_test]
async fn handle() {
	let request = Flag::new();
	let response = Flag::new();
	let (sender, receiver) = oneshot::channel();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();

		|context| async move {
			sender.send(context.tls()).unwrap();

			// Worker will be terminated before the request signal is sent.
			request.await;
			response.signal();
		}
	});

	let tls = receiver.await.unwrap();

	assert!(worker.destroy(tls).is_ok());

	// The worker will never respond if destroyed.
	request.signal();
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// Calling [`WorkerHandle::destroy()`](wasm_worker::WorkerHandle::destroy)
/// twice on the same worker.
#[wasm_bindgen_test]
async fn handle_twice() {
	let (sender, receiver) = oneshot::channel();

	let worker = wasm_worker::spawn(|context| sender.send((context.tls(), context.tls())).unwrap());

	let (tls_1, tls_2) = receiver.await.unwrap();
	worker.clone().destroy(tls_1).unwrap();

	assert!(matches!(
		worker.destroy(tls_2),
		Err(DestroyError::Already(_))
	));
}

/// Calling [`WorkerHandle::destroy()`](wasm_worker::WorkerHandle::destroy)
/// with the wrong [`Tls`](wasm_worker::Tls).
#[wasm_bindgen_test]
async fn handle_wrong() {
	let (sender_wrong, receiver_wrong) = oneshot::channel();
	let (sender_right, receiver_right) = oneshot::channel();

	wasm_worker::spawn(|context| {
		sender_wrong.send((context.tls(), context.tls())).unwrap();
		context.close();
	});
	let worker =
		wasm_worker::spawn(|context| sender_right.send((context.tls(), context.tls())).unwrap());

	let (tls_wrong_1, tls_wrong_2) = receiver_wrong.await.unwrap();
	let (tls_right_1, tls_right_2) = receiver_right.await.unwrap();

	assert!(matches!(
		worker.clone().destroy(tls_wrong_1),
		Err(DestroyError::Match { .. })
	));
	assert!(matches!(worker.clone().destroy(tls_right_1), Ok(())));
	assert!(matches!(
		worker.clone().destroy(tls_wrong_2),
		Err(DestroyError::Already(_))
	));
	assert!(matches!(
		worker.destroy(tls_right_2),
		Err(DestroyError::Already(_))
	));
}

/// [`WorkerHandleRef::destroy()`](wasm_worker::WorkerHandleRef::destroy).
#[wasm_bindgen_test]
async fn handle_ref() -> Result<()> {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let mut response = Flag::new();
	let (sender, receiver) = oneshot::channel();
	let receiver = Rc::new(RefCell::new(receiver));

	let _worker = WorkerBuilder::new()?
		.message_handler_async({
			let request = request.clone();
			move |worker, _| {
				let request = request.clone();
				let receiver = Rc::clone(&receiver);
				let worker = worker.clone();

				async move {
					let tls = RefCell::borrow_mut(&receiver).deref_mut().await.unwrap();

					assert!(worker.destroy(tls).is_ok());

					request.signal();
				}
			}
		})
		.spawn_async({
			let request = request.clone();
			let response = response.clone();

			|context| async move {
				sender.send(context.tls()).unwrap();
				context
					.transfer_messages([Message::ArrayBuffer(ArrayBuffer::new(1))])
					.unwrap();

				// Worker will be terminated before the request signal is sent.
				request.await;
				response.signal();
			}
		});

	// The worker will never respond if destroyed.
	let result = future::select(&mut response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));

	Ok(())
}

/// Calling [`WorkerHandleRef::destroy()`](wasm_worker::WorkerHandleRef::destroy)
/// twice on the same worker.
#[wasm_bindgen_test]
async fn handle_ref_twice() -> Result<()> {
	let flag = Flag::new();
	let (sender, receiver) = oneshot::channel();
	let receiver = Rc::new(RefCell::new(receiver));

	let _worker = WorkerBuilder::new()?
		.message_handler_async({
			let flag = flag.clone();
			let receiver = Rc::clone(&receiver);

			move |worker, _| {
				let flag = flag.clone();
				let receiver = Rc::clone(&receiver);
				let worker = worker.clone();

				async move {
					let (tls_1, tls_2) = RefCell::borrow_mut(&receiver).deref_mut().await.unwrap();
					worker.clone().destroy(tls_1).unwrap();

					assert!(matches!(
						worker.destroy(tls_2),
						Err(DestroyError::Already(_))
					));

					// Flag will never signal if `assert!` panics.
					flag.signal();
				}
			}
		})
		.spawn(|context| {
			sender.send((context.tls(), context.tls())).unwrap();
			context
				.transfer_messages([Message::ArrayBuffer(ArrayBuffer::new(1))])
				.unwrap();
		});

	flag.await;

	Ok(())
}

/// Calling [`WorkerHandleRef::destroy()`](wasm_worker::WorkerHandleRef::destroy)
/// with the wrong [`Tls`](wasm_worker::Tls).
#[wasm_bindgen_test]
async fn handle_ref_wrong() -> Result<()> {
	let flag = Flag::new();
	let (sender_wrong, receiver_wrong) = oneshot::channel();
	let (sender_right, receiver_right) = oneshot::channel();
	let receiver_wrong = Rc::new(RefCell::new(receiver_wrong));
	let receiver_right = Rc::new(RefCell::new(receiver_right));

	wasm_worker::spawn(|context| {
		sender_wrong.send((context.tls(), context.tls())).unwrap();
		context.close();
	});
	let _worker = WorkerBuilder::new()?
		.message_handler_async({
			let flag = flag.clone();
			let receiver_wrong = Rc::clone(&receiver_wrong);
			let receiver_right = Rc::clone(&receiver_right);

			move |worker, _| {
				let flag = flag.clone();
				let receiver_wrong = Rc::clone(&receiver_wrong);
				let receiver_right = Rc::clone(&receiver_right);
				let worker = worker.clone();

				async move {
					let (tls_wrong_1, tls_wrong_2) = RefCell::borrow_mut(&receiver_wrong)
						.deref_mut()
						.await
						.unwrap();
					let (tls_right_1, tls_right_2) = RefCell::borrow_mut(&receiver_right)
						.deref_mut()
						.await
						.unwrap();

					assert!(matches!(
						worker.clone().destroy(tls_wrong_1),
						Err(DestroyError::Match { .. })
					));
					assert!(matches!(worker.clone().destroy(tls_right_1), Ok(())));
					assert!(matches!(
						worker.clone().destroy(tls_wrong_2),
						Err(DestroyError::Already(_))
					));
					assert!(matches!(
						worker.destroy(tls_right_2),
						Err(DestroyError::Already(_))
					));

					// Flag will never signal if `assert!` panics.
					flag.signal();
				}
			}
		})
		.spawn(|context| {
			sender_right.send((context.tls(), context.tls())).unwrap();
			context
				.transfer_messages([Message::ArrayBuffer(ArrayBuffer::new(1))])
				.unwrap();
		});

	flag.await;

	Ok(())
}
