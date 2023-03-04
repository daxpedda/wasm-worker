use js_sys::ArrayBuffer;
use once_cell::sync::OnceCell;

use crate::global::{Global, WindowOrWorker};
use crate::message::MessageSupportError;

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if let WindowOrWorker::Worker(_) = global {
					if Global::new().worker().is_undefined() {
						return Err(MessageSupportError);
					}
				}

				let buffer = ArrayBuffer::new(1);

				Ok(super::test_support(&buffer))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
