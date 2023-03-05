use once_cell::sync::OnceCell;
use web_sys::MessageChannel;

use super::super::MessageSupportError;
use crate::global::{Global, WindowOrWorker};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if let WindowOrWorker::Worker(_) = global {
					if !Global::has_worker() {
						return Err(MessageSupportError);
					}
				}

				let channel = MessageChannel::new().unwrap();
				let port = channel.port1();

				Ok(super::test_support(&port))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
