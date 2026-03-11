use crate::config::LanguageDetectionConfig;
use crate::lid::{
    choose_inference_language, DecisionReason, DetectionCandidate, InferenceLanguageDecision,
};
use crate::logging::unix_ms;
use std::time::Instant;

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
        let (decision, retry_language_override) = if self.lid_config.enabled {
            let started = Instant::now();
            let candidates = self.detector.detect_language(pcm_s16le_mono_16k);
            eprintln!(
                "[ts_ms={}][DEBUG] detect_language took_ms={}",
                unix_ms(),
                started.elapsed().as_millis()
            );
            let decision = choose_inference_language(self.lid_config, &candidates);
            let language = if decision.reason == DecisionReason::SelectedFromDetection {
                Some(decision.selected_language.clone())
            } else {
                None
            };
            (decision, language)
        } else {
            (
                InferenceLanguageDecision {
                    selected_language: self.lid_config.default_language.clone(),
                    reason: DecisionReason::LidDisabled,
                },
                None::<String>,
            )
        };

        let forced_language_override = if self.lid_config.use_detected_language_for_inference {
            Some(decision.selected_language.as_str())
        } else if !self.lid_config.enabled {
            Some(self.lid_config.default_language.as_str())
        } else {
            None
        };

        let started = Instant::now();
        let mut text = self
            .transcriber
            .transcribe(pcm_s16le_mono_16k, forced_language_override)?;
        eprintln!(
            "[ts_ms={}][DEBUG] transcribe language={} took_ms={}",
            unix_ms(),
            forced_language_override.unwrap_or("auto"),
            started.elapsed().as_millis()
        );

        if text.trim().is_empty()
            && !self.lid_config.use_detected_language_for_inference
            && matches!(decision.reason, DecisionReason::SelectedFromDetection)
        {
            if let Some(language) = retry_language_override.as_deref() {
                eprintln!(
                    "[ts_ms={}][DEBUG] retry transcription with detected language={language}",
                    unix_ms()
                );
                let retry_started = Instant::now();
                text = self.transcriber.transcribe(pcm_s16le_mono_16k, Some(language))?;
                eprintln!(
                    "[ts_ms={}][DEBUG] transcribe language={} took_ms={}",
                    unix_ms(),
                    language,
                    retry_started.elapsed().as_millis()
                );
            }
        }

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
        pub responses: RefCell<Vec<String>>,
    }

    impl WhisperTranscriber for RecordingTranscriber {
        fn transcribe(
            &self,
            _pcm_s16le_mono_16k: &[i16],
            language: Option<&str>,
        ) -> Result<String, TranscriptionError> {
            *self.language_received.borrow_mut() = language.map(str::to_owned);
            Ok(self
                .responses
                .borrow_mut()
                .drain(..1)
                .next()
                .unwrap_or_else(|| "hello".to_owned()))
        }
    }

    #[test]
    fn passes_detected_language_to_transcriber() {
        let cfg = LanguageDetectionConfig {
            enabled: true,
            use_detected_language_for_inference: true,
            ..LanguageDetectionConfig::default()
        };
        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "en".to_owned(),
                confidence: 0.92,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
            responses: RefCell::new(vec!["hello".to_owned()]),
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
        let cfg = LanguageDetectionConfig {
            enabled: true,
            use_detected_language_for_inference: true,
            ..LanguageDetectionConfig::default()
        };
        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "ja".to_owned(),
                confidence: 0.99,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
            responses: RefCell::new(vec!["hello".to_owned()]),
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
            enabled: false,
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
            responses: RefCell::new(vec!["hello".to_owned(), "hello".to_owned()]),
        };

        let processor = SessionProcessor {
            detector: &detector,
            transcriber: &transcriber,
            lid_config: &cfg,
        };

        let _ = processor.process_session(&[1_i16]).expect("process");
        assert_eq!(
            transcriber.language_received.borrow().clone(),
            Some("zh".to_owned())
        );
    }

    #[test]
    fn retries_with_detected_language_after_empty_auto_result() {
        let cfg = LanguageDetectionConfig::default();
        let cfg = LanguageDetectionConfig {
            enabled: true,
            ..cfg
        };
        let detector = FakeDetector {
            candidates: vec![DetectionCandidate {
                language: "zh".to_owned(),
                confidence: 0.99,
            }],
        };
        let transcriber = RecordingTranscriber {
            language_received: RefCell::new(None),
            responses: RefCell::new(vec![String::new(), "ni hao".to_owned()]),
        };

        let processor = SessionProcessor {
            detector: &detector,
            transcriber: &transcriber,
            lid_config: &cfg,
        };

        let output = processor.process_session(&[1_i16, 2, 3]).expect("process");
        assert_eq!(output.text, "ni hao");
        assert_eq!(
            transcriber.language_received.borrow().clone(),
            Some("zh".to_owned())
        );
    }
}
