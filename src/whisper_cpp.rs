use crate::config::WhisperCppConfig;
use crate::lid::DetectionCandidate;
use crate::pipeline::{LanguageDetector, TranscriptionError, WhisperTranscriber};
use std::panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperCppEngine {
    cfg: WhisperCppConfig,
    ctx: WhisperContext,
}

impl WhisperCppEngine {
    pub fn new(cfg: WhisperCppConfig) -> Result<Self, TranscriptionError> {
        let ctx = create_context(&cfg)?;
        Ok(Self { cfg, ctx })
    }

    fn pcm_i16_to_f32(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<f32> {
        let gain = input_gain(pcm_s16le_mono_16k);
        if gain > 1.0 {
            eprintln!("[DEBUG] whisper input_gain={gain:.2}");
        }

        pcm_s16le_mono_16k
            .iter()
            .map(|s| ((*s as f32 / i16::MAX as f32) * gain).clamp(-1.0, 1.0))
            .collect()
    }

    fn run_full(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<(String, Option<String>, Option<f32>), TranscriptionError> {
        let audio = self.pcm_i16_to_f32(pcm_s16le_mono_16k);
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| TranscriptionError::Engine(format!("create whisper state failed: {e}")))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.cfg.threads as i32);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_no_timestamps(true);
        params.set_single_segment(true);
        params.set_suppress_blank(false);
        params.set_temperature(self.cfg.temperature);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        if let Some(lang) = language {
            params.set_language(Some(lang));
            params.set_detect_language(false);
        } else {
            params.set_language(None);
            params.set_detect_language(true);
        }

        state
            .full(params, &audio)
            .map_err(|e| TranscriptionError::Engine(format!("whisper full run failed: {e}")))?;

        let n = state.full_n_segments();
        eprintln!(
            "[DEBUG] whisper requested_language={} segments={}",
            language.unwrap_or("auto"),
            n
        );

        let mut text = String::new();
        for i in 0..n {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| {
                    TranscriptionError::Engine(format!("read segment {i} failed: out of bounds"))
                })?;
            let seg = segment.to_str().map_err(|e| {
                TranscriptionError::Engine(format!("read segment {i} failed: {e}"))
            })?;
            text.push_str(seg.trim());
            if i + 1 < n {
                text.push(' ');
            }
        }

        let lang_id = match state.full_lang_id_from_state() {
            id if id >= 0 => Some(id),
            _ => None,
        };
        let lang = lang_id.and_then(|id| whisper_rs::get_lang_str(id).map(str::to_owned));
        let confidence = match lang_id {
            Some(id) => match safe_lang_detect(&mut state, self.cfg.threads) {
                Ok(probs) => probs.get(id as usize).copied().or(Some(1.0)),
                _ => Some(1.0),
            },
            None => None,
        };
        eprintln!(
            "[DEBUG] whisper detected_language={} confidence={:?} text_len={}",
            lang.as_deref().unwrap_or("unknown"),
            confidence,
            text.trim().len()
        );

        Ok((text, lang, confidence))
    }
}

fn create_context(cfg: &WhisperCppConfig) -> Result<WhisperContext, TranscriptionError> {
    let mut params = WhisperContextParameters::default();
    params.use_gpu = cfg.use_gpu;

    WhisperContext::new_with_params(&cfg.model, params)
        .map_err(|e| TranscriptionError::Engine(format!("init whisper context failed: {e}")))
}

fn safe_lang_detect(
    state: &mut whisper_rs::WhisperState,
    threads: usize,
) -> Result<Vec<f32>, Box<dyn std::any::Any + Send>> {
    let hook = take_hook();
    set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| state.lang_detect(0, threads)));
    set_hook(hook);

    match result {
        Ok(inner) => Ok(inner.map(|(_, probs)| probs).unwrap_or_default()),
        Err(err) => Err(err),
    }
}

fn input_gain(pcm_s16le_mono_16k: &[i16]) -> f32 {
    let peak = pcm_s16le_mono_16k
        .iter()
        .map(|sample| sample.saturating_abs() as i32)
        .max()
        .unwrap_or(0);

    if peak < 256 {
        return 1.0;
    }

    let normalized_peak = peak as f32 / i16::MAX as f32;
    let target_peak = 0.85_f32;
    let max_gain = 6.0_f32;

    (target_peak / normalized_peak).clamp(1.0, max_gain)
}

impl LanguageDetector for WhisperCppEngine {
    fn detect_language(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<DetectionCandidate> {
        let Ok((_, maybe_lang, maybe_conf)) = self.run_full(pcm_s16le_mono_16k, None) else {
            return Vec::new();
        };

        match maybe_lang {
            Some(language) => vec![DetectionCandidate {
                language,
                confidence: maybe_conf.unwrap_or(0.0),
            }],
            None => Vec::new(),
        }
    }
}

impl WhisperTranscriber for WhisperCppEngine {
    fn transcribe(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let (text, _, _) = self.run_full(pcm_s16le_mono_16k, language)?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_gain_boosts_low_level_audio_with_limit() {
        assert_eq!(input_gain(&[]), 1.0);
        assert_eq!(input_gain(&[100, -200]), 1.0);
        assert!((input_gain(&[2_000, -2_000]) - 6.0).abs() < f32::EPSILON);
        assert!(input_gain(&[10_000, -10_000]) > 1.0);
        assert!(input_gain(&[30_000, -30_000]) <= 1.0);
    }
}
