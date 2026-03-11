use crate::config::WhisperCppConfig;
use crate::lid::DetectionCandidate;
use crate::pipeline::{LanguageDetector, TranscriptionError, WhisperTranscriber};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, set_hook, take_hook, AssertUnwindSafe};
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperCppEngine {
    cfg: WhisperCppConfig,
    ctx: WhisperContext,
    cache: Mutex<Option<CachedRun>>,
}

#[derive(Clone)]
struct CachedRun {
    audio_hash: u64,
    requested_language: Option<String>,
    text: String,
    detected_language: Option<String>,
    confidence: Option<f32>,
}

impl WhisperCppEngine {
    pub fn new(cfg: WhisperCppConfig) -> Result<Self, TranscriptionError> {
        let ctx = create_context(&cfg)?;
        Ok(Self {
            cfg,
            ctx,
            cache: Mutex::new(None),
        })
    }

    fn pcm_i16_to_f32(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<f32> {
        pcm_s16le_mono_16k
            .iter()
            .map(|s| *s as f32 / i16::MAX as f32)
            .collect()
    }

    fn audio_hash(&self, pcm_s16le_mono_16k: &[i16]) -> u64 {
        let mut hasher = DefaultHasher::new();
        pcm_s16le_mono_16k.hash(&mut hasher);
        hasher.finish()
    }

    fn run_full(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<(String, Option<String>, Option<f32>), TranscriptionError> {
        let audio_hash = self.audio_hash(pcm_s16le_mono_16k);
        if let Some(cached) = self.lookup_cache(audio_hash, language) {
            return Ok((cached.text, cached.detected_language, cached.confidence));
        }

        let audio = self.pcm_i16_to_f32(pcm_s16le_mono_16k);
        let mut state = self
            .ctx
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

        let lang_id = state.full_lang_id_from_state().ok();
        let lang = lang_id.and_then(|id| whisper_rs::get_lang_str(id).map(str::to_owned));
        let confidence = match lang_id {
            Some(id) => match safe_lang_detect(&mut state, self.cfg.threads) {
                Ok(probs) => probs.get(id as usize).copied().or(Some(1.0)),
                _ => Some(1.0),
            },
            None => None,
        };

        self.store_cache(CachedRun {
            audio_hash,
            requested_language: language.map(str::to_owned),
            text: text.clone(),
            detected_language: lang.clone(),
            confidence,
        });

        Ok((text, lang, confidence))
    }

    fn lookup_cache(&self, audio_hash: u64, language: Option<&str>) -> Option<CachedRun> {
        let cached = self.cache.lock().ok()?.clone()?;
        if cached.audio_hash != audio_hash {
            return None;
        }

        match (&cached.requested_language, language) {
            (Some(cached_lang), Some(requested_lang)) if cached_lang == requested_lang => Some(cached),
            (None, None) => Some(cached),
            (None, Some(requested_lang))
                if cached.detected_language.as_deref() == Some(requested_lang) =>
            {
                Some(cached)
            }
            _ => None,
        }
    }

    fn store_cache(&self, run: CachedRun) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(run);
        }
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
        Ok(inner) => Ok(inner.unwrap_or_default()),
        Err(err) => Err(err),
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
