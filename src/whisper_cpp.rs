use crate::config::WhisperCppConfig;
use crate::lid::DetectionCandidate;
use crate::pipeline::{LanguageDetector, TranscriptionError, WhisperTranscriber};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperCppEngine {
    cfg: WhisperCppConfig,
}

impl WhisperCppEngine {
    pub fn new(cfg: WhisperCppConfig) -> Self {
        Self { cfg }
    }

    fn pcm_i16_to_f32(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<f32> {
        pcm_s16le_mono_16k
            .iter()
            .map(|s| *s as f32 / i16::MAX as f32)
            .collect()
    }

    fn create_context(&self) -> Result<WhisperContext, TranscriptionError> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu = self.cfg.use_gpu;
        params.flash_attn = self.cfg.flash_attn;

        WhisperContext::new_with_params(&self.cfg.model, params)
            .map_err(|e| TranscriptionError::Engine(format!("init whisper context failed: {e}")))
    }

    fn run_full(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<(String, Option<String>, Option<f32>), TranscriptionError> {
        let audio = self.pcm_i16_to_f32(pcm_s16le_mono_16k);
        let ctx = self.create_context()?;
        let mut state = ctx
            .create_state()
            .map_err(|e| TranscriptionError::Engine(format!("create whisper state failed: {e}")))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.cfg.threads as i32);
        params.set_translate(false);
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

        let n = state
            .full_n_segments()
            .map_err(|e| TranscriptionError::Engine(format!("read segment count failed: {e}")))?;

        let mut text = String::new();
        for i in 0..n {
            let seg = state
                .full_get_segment_text(i)
                .map_err(|e| TranscriptionError::Engine(format!("read segment {i} failed: {e}")))?;
            text.push_str(seg.trim());
            if i + 1 < n {
                text.push(' ');
            }
        }

        let lang_id = state.full_lang_id().ok();
        let lang = lang_id.and_then(|id| whisper_rs::get_lang_str(id).map(str::to_owned));
        let confidence = lang_id.and_then(|id| state.lang_detect_probs(id).ok());

        Ok((text, lang, confidence))
    }
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
