use js_sys::ArrayBuffer;
use once_cell::sync::OnceCell;
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat};

use super::super::MessageSupportError;
use crate::global::{Global, WindowOrWorker};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if Global::with(Global::audio_data).is_undefined() {
					return Ok(false);
				}

				if let WindowOrWorker::Worker(_) = global {
					if !Global::has_worker() {
						return Err(MessageSupportError);
					}
				}

				let init = AudioDataInit::new(
					&ArrayBuffer::new(1),
					AudioSampleFormat::U8,
					1,
					1,
					3000.,
					0.,
				);
				let data = AudioData::new(&init).unwrap();

				Ok(super::test_support(&data))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
