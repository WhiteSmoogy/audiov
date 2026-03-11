use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub language_detection: LanguageDetectionConfig,
    #[serde(default)]
    pub whisper_cpp: WhisperCppConfig,
    #[serde(default)]
    pub whisper_remote: WhisperRemoteConfig,
    #[serde(default)]
    pub whisper: WhisperConfig,
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    #[serde(default)]
    pub paste: PasteConfig,
    #[serde(default)]
    pub recorder: RecorderConfig,
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct WhisperRemoteConfig {
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_remote_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_remote_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_remote_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for WhisperRemoteConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            endpoint: default_remote_endpoint(),
            model: default_remote_model(),
            api_key: String::new(),
            timeout_secs: default_remote_timeout_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct WhisperConfig {
    #[serde(default = "default_whisper_backend")]
    pub backend: String,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            backend: default_whisper_backend(),
        }
    }
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

fn default_false() -> bool {
    false
}

fn default_remote_endpoint() -> String {
    "https://api.openai.com/v1/audio/transcriptions".to_owned()
}

fn default_remote_model() -> String {
    "whisper-1".to_owned()
}

fn default_remote_timeout_secs() -> u64 {
    60
}

fn default_whisper_backend() -> String {
    "cpp".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HotkeyConfig {
    #[serde(default = "default_hotkey")]
    pub key: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            key: default_hotkey(),
        }
    }
}

fn default_hotkey() -> String {
    "windows+h".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PasteConfig {
    #[serde(default = "default_paste_command")]
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RecorderConfig {
    #[serde(default = "default_recorder_backend")]
    pub backend: String,
    #[serde(default)]
    pub input_device: Option<String>,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            backend: default_recorder_backend(),
            input_device: None,
        }
    }
}

fn default_recorder_backend() -> String {
    "auto".to_owned()
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self {
            command: default_paste_command(),
        }
    }
}

fn default_paste_command() -> Vec<String> {
    vec![
        "ydotool".to_owned(),
        "key".to_owned(),
        "29:1".to_owned(),
        "47:1".to_owned(),
        "47:0".to_owned(),
        "29:0".to_owned(),
    ]
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
        assert_eq!(cfg.whisper_remote, WhisperRemoteConfig::default());
        assert_eq!(cfg.whisper, WhisperConfig::default());
        assert_eq!(cfg.hotkey, HotkeyConfig::default());
        assert_eq!(cfg.paste, PasteConfig::default());
        assert_eq!(cfg.recorder, RecorderConfig::default());
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

            [whisper_remote]
            enabled = true
            endpoint = "https://example.com/v1/audio/transcriptions"
            model = "whisper-1"
            api_key = "test-key"
            timeout_secs = 30

            [whisper]
            backend = "remote"

            [hotkey]
            key = "f9"

            [paste]
            command = ["ydotool", "key", "29:1", "47:1", "47:0", "29:0"]

            [recorder]
            backend = "pipewire"
            input_device = "alsa_input.pci-0000_00_1f.3.analog-stereo"
            "#,
        )
        .expect("write config");

        let cfg = AppConfig::load_from_path(file.path()).expect("load");

        assert!(cfg.language_detection.enabled);
        assert_eq!(cfg.language_detection.confidence_threshold, 0.70);
        assert_eq!(cfg.whisper_cpp.model, "models/ggml-small.bin");
        assert_eq!(cfg.whisper_cpp.threads, 6);
        assert!(cfg.whisper_cpp.use_gpu);
        assert!(cfg.whisper_remote.enabled);
        assert_eq!(cfg.whisper_remote.api_key, "test-key");
        assert_eq!(cfg.whisper.backend, "remote");
        assert_eq!(cfg.hotkey.key, "f9");
        assert_eq!(cfg.paste.command[0], "ydotool");
        assert_eq!(cfg.recorder.backend, "pipewire");
        assert_eq!(
            cfg.recorder.input_device.as_deref(),
            Some("alsa_input.pci-0000_00_1f.3.analog-stereo")
        );
    }
}
