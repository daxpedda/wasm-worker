use once_cell::sync::OnceCell;
use web_sys::RtcPeerConnection;

use super::super::MessageSupportError;
use crate::global::{Global, GlobalContext};

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

			let connection = RtcPeerConnection::new().unwrap();
			let channel = connection.create_data_channel("");

			Ok(super::test_support(&channel))
		})
		.copied()
}
