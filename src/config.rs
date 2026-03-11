use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub language_detection: LanguageDetectionConfig,
    #[serde(default)]
    pub whisper_cpp: WhisperCppConfig,
}

impl AppConfig {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let raw = fs::read_to_string(path).map_err(ConfigError::Io)?;
        toml::from_str(&raw).map_err(ConfigError::Parse)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LanguageDetectionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_allowed_languages")]
    pub allowed_languages: Vec<String>,
    #[serde(default = "default_true")]
    pub use_detected_language_for_inference: bool,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,
    #[serde(default = "default_language")]
    pub default_language: String,
}

impl Default for LanguageDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            mode: default_mode(),
            allowed_languages: default_allowed_languages(),
            use_detected_language_for_inference: default_true(),
            confidence_threshold: default_confidence_threshold(),
            default_language: default_language(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_mode() -> String {
    "per_session".to_owned()
}

fn default_allowed_languages() -> Vec<String> {
    vec!["zh".to_owned(), "en".to_owned()]
}

fn default_confidence_threshold() -> f32 {
    0.65
}

fn default_language() -> String {
    "zh".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct WhisperCppConfig {
    #[serde(default = "default_whisper_model")]
    pub model: String,
    #[serde(default = "default_whisper_threads")]
    pub threads: usize,
    #[serde(default = "default_whisper_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub use_gpu: bool,
}

impl Default for WhisperCppConfig {
    fn default() -> Self {
        Self {
            model: default_whisper_model(),
            threads: default_whisper_threads(),
            temperature: default_whisper_temperature(),
            use_gpu: false,
        }
    }
}

fn default_whisper_model() -> String {
    "models/ggml-base.bin".to_owned()
}

fn default_whisper_threads() -> usize {
    4
}

fn default_whisper_temperature() -> f32 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_defaults_from_empty_lid_config() {
        let content = r#"[language_detection]
[whisper_cpp]"#;
        let cfg: AppConfig = toml::from_str(content).expect("parse");

        assert_eq!(cfg.language_detection, LanguageDetectionConfig::default());
        assert_eq!(cfg.whisper_cpp, WhisperCppConfig::default());
    }

    #[test]
    fn load_from_file() {
        let mut file = NamedTempFile::new().expect("create temp");
        std::io::Write::write_all(
            &mut file,
            br#"
            [language_detection]
            enabled = true
            allowed_languages = ["zh", "en"]
            confidence_threshold = 0.70
            default_language = "zh"

            [whisper_cpp]
            model = "models/ggml-small.bin"
            threads = 6
            temperature = 0.2
            use_gpu = true
            "#,
        )
        .expect("write config");

        let cfg = AppConfig::load_from_path(file.path()).expect("load");

        assert!(cfg.language_detection.enabled);
        assert_eq!(cfg.language_detection.confidence_threshold, 0.70);
        assert_eq!(cfg.whisper_cpp.model, "models/ggml-small.bin");
        assert_eq!(cfg.whisper_cpp.threads, 6);
        assert!(cfg.whisper_cpp.use_gpu);
    }
}
