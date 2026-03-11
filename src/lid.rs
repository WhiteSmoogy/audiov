use crate::config::LanguageDetectionConfig;

#[derive(Debug, Clone, PartialEq)]
pub struct DetectionCandidate {
    pub language: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceLanguageDecision {
    pub selected_language: String,
    pub reason: DecisionReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionReason {
    LidDisabled,
    DetectionUnavailable,
    CandidateBelowThreshold,
    CandidateNotWhitelisted,
    SelectedFromDetection,
}

pub fn choose_inference_language(
    lid_config: &LanguageDetectionConfig,
    candidates: &[DetectionCandidate],
) -> InferenceLanguageDecision {
    if !lid_config.enabled || !lid_config.use_detected_language_for_inference {
        return InferenceLanguageDecision {
            selected_language: lid_config.default_language.clone(),
            reason: DecisionReason::LidDisabled,
        };
    }

    let best = candidates
        .iter()
        .filter(|c| {
            lid_config
                .allowed_languages
                .iter()
                .any(|l| l == &c.language)
        })
        .max_by(|a, b| a.confidence.total_cmp(&b.confidence));

    let Some(best) = best else {
        return InferenceLanguageDecision {
            selected_language: lid_config.default_language.clone(),
            reason: if candidates.is_empty() {
                DecisionReason::DetectionUnavailable
            } else {
                DecisionReason::CandidateNotWhitelisted
            },
        };
    };

    if best.confidence < lid_config.confidence_threshold {
        return InferenceLanguageDecision {
            selected_language: lid_config.default_language.clone(),
            reason: DecisionReason::CandidateBelowThreshold,
        };
    }

    InferenceLanguageDecision {
        selected_language: best.language.clone(),
        reason: DecisionReason::SelectedFromDetection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LanguageDetectionConfig;

    #[test]
    fn chooses_whitelisted_lang_above_threshold() {
        let cfg = LanguageDetectionConfig::default();
        let decision = choose_inference_language(
            &cfg,
            &[DetectionCandidate {
                language: "en".into(),
                confidence: 0.91,
            }],
        );

        assert_eq!(decision.selected_language, "en");
        assert_eq!(decision.reason, DecisionReason::SelectedFromDetection);
    }

    #[test]
    fn falls_back_when_below_threshold() {
        let cfg = LanguageDetectionConfig::default();
        let decision = choose_inference_language(
            &cfg,
            &[DetectionCandidate {
                language: "en".into(),
                confidence: 0.30,
            }],
        );

        assert_eq!(decision.selected_language, "zh");
        assert_eq!(decision.reason, DecisionReason::CandidateBelowThreshold);
    }

    #[test]
    fn falls_back_when_not_whitelisted() {
        let cfg = LanguageDetectionConfig::default();
        let decision = choose_inference_language(
            &cfg,
            &[DetectionCandidate {
                language: "ja".into(),
                confidence: 0.99,
            }],
        );

        assert_eq!(decision.selected_language, "zh");
        assert_eq!(decision.reason, DecisionReason::CandidateNotWhitelisted);
    }

    #[test]
    fn falls_back_when_detection_missing() {
        let cfg = LanguageDetectionConfig::default();
        let decision = choose_inference_language(&cfg, &[]);

        assert_eq!(decision.selected_language, "zh");
        assert_eq!(decision.reason, DecisionReason::DetectionUnavailable);
    }
}
