use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::task::AtomicWaker;

#[derive(Clone)]
pub(crate) struct Flag(Arc<Inner>);

struct Inner {
	waker: AtomicWaker,
	value: AtomicU8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum State {
	None,
	Raised,
	Completed,
}

impl Debug for Flag {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_tuple("Flag").field(&self.load()).finish()
	}
}

impl Future for Flag {
	type Output = State;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let state = self.load();

		if state != State::None {
			return Poll::Ready(state);
		}

		self.0.waker.register(cx.waker());

		let state = self.load();

		if state == State::None {
			Poll::Pending
		} else {
			Poll::Ready(state)
		}
	}
}

impl Flag {
	pub(crate) fn new() -> Self {
		Self(Arc::new(Inner {
			waker: AtomicWaker::new(),
			value: AtomicU8::new(State::None.to_u8()),
		}))
	}

	fn load(&self) -> State {
		State::from_u8(self.0.value.load(Ordering::Relaxed))
	}

	fn set(&self, state: State) -> State {
		State::from_u8(self.0.value.fetch_max(state.to_u8(), Ordering::Relaxed))
	}

	pub(crate) fn raise(&self) {
		if self.set(State::Raised) != State::Raised {
			self.0.waker.wake();
		}
	}

	pub(crate) fn complete(&self) {
		if self.set(State::Completed) != State::Completed {
			self.0.waker.wake();
		}
	}

	pub(crate) fn is_completed(&self) -> bool {
		self.load() == State::Completed
	}
}

impl State {
	const fn to_u8(self) -> u8 {
		match self {
			Self::None => 0,
			Self::Raised => 1,
			Self::Completed => 2,
		}
	}

	fn from_u8(state: u8) -> Self {
		match state {
			0 => Self::None,
			1 => Self::Raised,
			2 => Self::Completed,
			_ => panic!("unexpected state"),
		}
	}
}
