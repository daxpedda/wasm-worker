use once_cell::sync::OnceCell;
use web_sys::RtcPeerConnection;

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

				let connection = RtcPeerConnection::new().unwrap();
				let channel = connection.create_data_channel("");

				Ok(super::test_support(&channel))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
