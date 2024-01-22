//! Parker implementation inspired by Std but adapted to non-threaded
//! environment.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use js_sys::Atomics;

use super::ZERO_ARRAY;

/// Parker implementation.
#[derive(Debug)]
pub(in super::super) struct Parker(AtomicBool);

impl Parker {
	/// Creates a new [`Parker`].
	#[allow(clippy::missing_const_for_fn)]
	pub(in super::super) fn new() -> Self {
		Self(AtomicBool::new(false))
	}

	/// Parks the thread.
	pub(in super::super) unsafe fn park(&self) {
		if self.0.swap(false, Ordering::Relaxed) {
			return;
		}

		wait(None);
		unreachable!("thread should have never woken up");
	}

	/// Parks the thread with a timeout.
	pub(in super::super) unsafe fn park_timeout(&self, timeout: Duration) {
		if self.0.swap(false, Ordering::Relaxed) {
			return;
		}

		wait(Some(timeout));
	}

	/// Unparks the thread.
	pub(in super::super) fn unpark(&self) {
		self.0.store(true, Ordering::Relaxed);
	}
}

/// Wait a specified duration.
fn wait(timeout: Option<Duration>) {
	#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
	let timeout = timeout.map_or(f64::INFINITY, |timeout| timeout.as_millis() as f64);

	let result = ZERO_ARRAY
		.with(|array| {
			let Some(array) = array else {
				unreachable!("forgot to check wait support first");
			};
			Atomics::wait_with_timeout(array, 0, 0, timeout)
		})
		.expect("`Atomic.wait` is not expected to fail");
	debug_assert_eq!(
		result, "timed-out",
		"unexpected return value from `Atomics.wait"
	);
}