//! `EncoderType` → `EncoderBackend` (breaks config ↔ video import cycles).

use crate::config::EncoderType;
use crate::video::codec::EncoderBackend;

/// `None` means “auto” in WebRTC / pipeline (same as `EncoderType::Auto`).
pub fn encoder_type_to_backend(encoder: EncoderType) -> Option<EncoderBackend> {
    match encoder {
        EncoderType::Auto => None,
        EncoderType::Software => Some(EncoderBackend::Software),
        EncoderType::Vaapi => Some(EncoderBackend::Vaapi),
        EncoderType::Nvenc => Some(EncoderBackend::Nvenc),
        EncoderType::Qsv => Some(EncoderBackend::Qsv),
        EncoderType::Amf => Some(EncoderBackend::Amf),
        EncoderType::Rkmpp => Some(EncoderBackend::Rkmpp),
        EncoderType::V4l2m2m => Some(EncoderBackend::V4l2m2m),
        EncoderType::Amlogic => Some(EncoderBackend::Amlogic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_amlogic_config_to_backend() {
        assert_eq!(
            encoder_type_to_backend(EncoderType::Amlogic),
            Some(EncoderBackend::Amlogic)
        );
    }

    #[test]
    fn amlogic_config_json_round_trip() {
        let json = serde_json::to_string(&EncoderType::Amlogic).unwrap();
        assert_eq!(json, "\"amlogic\"");
        assert_eq!(
            serde_json::from_str::<EncoderType>(&json).unwrap(),
            EncoderType::Amlogic
        );
    }
}
