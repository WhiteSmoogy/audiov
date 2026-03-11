use crate::config::LanguageDetectionConfig;
use crate::lid::{choose_inference_language, DetectionCandidate, InferenceLanguageDecision};

pub trait LanguageDetector {
    fn detect_language(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<DetectionCandidate>;
}

pub trait WhisperTranscriber {
    fn transcribe(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<String, TranscriptionError>;
}

#[derive(Debug)]
pub enum TranscriptionError {
    Engine(String),
}

#[derive(Debug)]
pub struct SessionOutput {
    pub text: String,
    pub language_decision: InferenceLanguageDecision,
}

pub struct SessionProcessor<'a, D, T> {
    pub detector: &'a D,
    pub transcriber: &'a T,
    pub lid_config: &'a LanguageDetectionConfig,
}

impl<'a, D, T> SessionProcessor<'a, D, T>
where
    D: LanguageDetector,
    T: WhisperTranscriber,
{
    pub fn process_session(
        &self,
        pcm_s16le_mono_16k: &[i16],
    ) -> Result<SessionOutput, TranscriptionError> {
        let candidates = if self.lid_config.enabled {
            self.detector.detect_language(pcm_s16le_mono_16k)
        } else {
            Vec::new()
        };

        let decision = choose_inference_language(self.lid_config, &candidates);
        let language_arg = if self.lid_config.use_detected_language_for_inference {
            Some(decision.selected_language.as_str())
        } else {
            None
        };

        let text = self
            .transcriber
            .transcribe(pcm_s16le_mono_16k, language_arg)?;

        Ok(SessionOutput {
            text,
            language_decision: decision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LanguageDetectionConfig;
    use std::cell::RefCell;

    struct FakeDetector {
        candidates: Vec<DetectionCandidate>,
    }

    impl LanguageDetector for FakeDetector {
        fn detect_language(&self, _pcm_s16le_mono_16k: &[i16]) -> Vec<DetectionCandidate> {
            self.candidates.clone()
        }
    }

    struct RecordingTranscriber {
        pub language_received: RefCell<Option<String>>,
    }

    impl WhisperTranscriber for RecordingTranscriber {
        fn transcribe(
            &self,
            _pcm_s16le_mono_16k: &[i16],
            language: Option<&str>,
        ) -> Result<String, TranscriptionError> {
            *self.language_received.borrow_mut() = language.map(str::to_owned);
            Ok("hello".to_owned())
        }
    }

    #[test]
    fn passes_detected_language_to_transcriber() {
        let cfg = LanguageDetectionConfig::default();
        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "en".to_owned(),
                confidence: 0.92,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
        };

        let processor = SessionProcessor {
            detector: &detector,
            transcriber: &transcriber,
            lid_config: &cfg,
        };

        let output = processor.process_session(&[1_i16, 2, 3]).expect("process");
        assert_eq!(output.text, "hello");
        assert_eq!(output.language_decision.selected_language, "en");
        assert_eq!(
            transcriber.language_received.borrow().clone(),
            Some("en".to_owned())
        );
    }

    #[test]
    fn falls_back_to_default_language_when_detection_not_whitelisted() {
        let cfg = LanguageDetectionConfig::default();
        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "ja".to_owned(),
                confidence: 0.99,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
        };

        let processor = SessionProcessor {
            detector: &detector,
            transcriber: &transcriber,
            lid_config: &cfg,
        };

        let output = processor.process_session(&[7_i16, 8]).expect("process");
        assert_eq!(output.language_decision.selected_language, "zh");
        assert_eq!(
            transcriber.language_received.borrow().clone(),
            Some("zh".to_owned())
        );
    }

    #[test]
    fn sends_no_language_when_disabled_for_inference() {
        let cfg = LanguageDetectionConfig {
            use_detected_language_for_inference: false,
            ..LanguageDetectionConfig::default()
        };

        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "en".to_owned(),
                confidence: 0.99,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
        };

        let processor = SessionProcessor {
            detector: &detector,
            transcriber: &transcriber,
            lid_config: &cfg,
        };

        let _ = processor.process_session(&[1_i16]).expect("process");
        assert_eq!(transcriber.language_received.borrow().clone(), None);
    }
}
