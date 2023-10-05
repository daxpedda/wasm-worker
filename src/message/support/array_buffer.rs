use js_sys::ArrayBuffer;
use once_cell::sync::OnceCell;

use crate::global::{Global, GlobalContext};
use crate::message::MessageSupportError;

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			GlobalContext::with(|global| match global {
				GlobalContext::Window(_) => Ok(()),
				GlobalContext::Worker(_) => {
					if Global::has_worker() {
						Ok(())
					} else {
						Err(MessageSupportError::Context)
					}
				}
				GlobalContext::Worklet => Err(MessageSupportError::Context),
			})?;

			let buffer = ArrayBuffer::new(1);

			Ok(super::test_support(&buffer))
		})
		.copied()
}
