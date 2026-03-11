use crate::config::WhisperRemoteConfig;
use crate::lid::DetectionCandidate;
use crate::pipeline::{LanguageDetector, TranscriptionError, WhisperTranscriber};
use reqwest::blocking::{multipart, Client};
use serde::Deserialize;
use std::time::Duration;

pub struct WhisperRemoteEngine {
    cfg: WhisperRemoteConfig,
    client: Client,
}

impl WhisperRemoteEngine {
    pub fn new(cfg: WhisperRemoteConfig) -> Result<Self, TranscriptionError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()
            .map_err(|e| TranscriptionError::Engine(format!("init remote client failed: {e}")))?;

        Ok(Self { cfg, client })
    }

    fn pcm_to_wav_bytes(&self, pcm_s16le_mono_16k: &[i16]) -> Result<Vec<u8>, TranscriptionError> {
        let mut bytes = Vec::new();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        {
            let cursor = std::io::Cursor::new(&mut bytes);
            let mut writer = hound::WavWriter::new(cursor, spec)
                .map_err(|e| TranscriptionError::Engine(format!("create wav failed: {e}")))?;
            for sample in pcm_s16le_mono_16k {
                writer.write_sample(*sample).map_err(|e| {
                    TranscriptionError::Engine(format!("write wav sample failed: {e}"))
                })?;
            }
            writer
                .finalize()
                .map_err(|e| TranscriptionError::Engine(format!("finalize wav failed: {e}")))?;
        }

        Ok(bytes)
    }

    fn remote_transcribe(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<RemoteTranscriptionResponse, TranscriptionError> {
        let wav = self.pcm_to_wav_bytes(pcm_s16le_mono_16k)?;
        let file_part = multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscriptionError::Engine(format!("build multipart failed: {e}")))?;

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.cfg.model.clone());

        if let Some(lang) = language {
            form = form.text("language", lang.to_owned());
        }

        let resp = self
            .client
            .post(&self.cfg.endpoint)
            .bearer_auth(&self.cfg.api_key)
            .multipart(form)
            .send()
            .map_err(|e| {
                TranscriptionError::Engine(format!("remote whisper request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .unwrap_or_else(|_| "<failed to read error body>".to_owned());
            return Err(TranscriptionError::Engine(format!(
                "remote whisper returned {status}: {body}"
            )));
        }

        resp.json::<RemoteTranscriptionResponse>().map_err(|e| {
            TranscriptionError::Engine(format!("parse remote whisper response failed: {e}"))
        })
    }
}

impl WhisperTranscriber for WhisperRemoteEngine {
    fn transcribe(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let response = self.remote_transcribe(pcm_s16le_mono_16k, language)?;
        Ok(response.text)
    }
}

impl LanguageDetector for WhisperRemoteEngine {
    fn detect_language(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<DetectionCandidate> {
        let Ok(response) = self.remote_transcribe(pcm_s16le_mono_16k, None) else {
            return Vec::new();
        };

        match response.language {
            Some(language) => vec![DetectionCandidate {
                language,
                confidence: response.language_probability.unwrap_or(0.0),
            }],
            None => Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RemoteTranscriptionResponse {
    text: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    language_probability: Option<f32>,
}
