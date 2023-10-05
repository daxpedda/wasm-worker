use js_sys::ArrayBuffer;
use once_cell::sync::OnceCell;
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat};

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

			if Global::with(Global::audio_data).is_undefined() {
				return Ok(false);
			}

			let init =
				AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
			let data = AudioData::new(&init).unwrap();

			Ok(super::test_support(&data))
		})
		.copied()
}
