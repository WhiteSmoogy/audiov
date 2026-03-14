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
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_allowed_languages")]
    pub allowed_languages: Vec<String>,
    #[serde(default = "default_false")]
    pub use_detected_language_for_inference: bool,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,
    #[serde(default = "default_language")]
    pub default_language: String,
}

impl Default for LanguageDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: default_false(),
            mode: default_mode(),
            allowed_languages: default_allowed_languages(),
            use_detected_language_for_inference: default_false(),
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
    "models/ggml-large-v1.bin".to_owned()
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
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_hotkey_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_hotkey_component_unique")]
    pub component_unique: String,
    #[serde(default = "default_hotkey_component_friendly")]
    pub component_friendly: String,
    #[serde(default = "default_hotkey_action_unique")]
    pub action_unique: String,
    #[serde(default = "default_hotkey_action_friendly")]
    pub action_friendly: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            shortcut: default_hotkey_shortcut(),
            component_unique: default_hotkey_component_unique(),
            component_friendly: default_hotkey_component_friendly(),
            action_unique: default_hotkey_action_unique(),
            action_friendly: default_hotkey_action_friendly(),
        }
    }
}

fn default_hotkey_shortcut() -> String {
    "Meta+H".to_owned()
}

fn default_hotkey_component_unique() -> String {
    "audiov".to_owned()
}

fn default_hotkey_component_friendly() -> String {
    "audiov".to_owned()
}

fn default_hotkey_action_unique() -> String {
    "toggle-recording".to_owned()
}

fn default_hotkey_action_friendly() -> String {
    "Toggle Recording".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PasteConfig {
    #[serde(default = "default_paste_mode")]
    pub mode: String,
    #[serde(default = "default_paste_command")]
    pub command: Vec<String>,
    #[serde(default = "default_fcitx5_service")]
    pub fcitx5_service: String,
    #[serde(default = "default_fcitx5_path")]
    pub fcitx5_path: String,
    #[serde(default = "default_fcitx5_interface")]
    pub fcitx5_interface: String,
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
            mode: default_paste_mode(),
            command: default_paste_command(),
            fcitx5_service: default_fcitx5_service(),
            fcitx5_path: default_fcitx5_path(),
            fcitx5_interface: default_fcitx5_interface(),
        }
    }
}

fn default_paste_mode() -> String {
    "command".to_owned()
}

fn default_paste_command() -> Vec<String> {
    vec![
        "ydotool".to_owned(),
        "key".to_owned(),
        "29:1".to_owned(),
        "42:1".to_owned(),
        "47:1".to_owned(),
        "47:0".to_owned(),
        "42:0".to_owned(),
        "29:0".to_owned(),
    ]
}

fn default_fcitx5_service() -> String {
    "org.fcitx.Fcitx5".to_owned()
}

fn default_fcitx5_path() -> String {
    "/org/freedesktop/Fcitx5/Audiov".to_owned()
}

fn default_fcitx5_interface() -> String {
    "org.fcitx.Fcitx5.Audiov1".to_owned()
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
            model = "models/ggml-large-v1.bin"
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
            enabled = true
            shortcut = "Meta+H"
            component_unique = "audiov"
            component_friendly = "audiov"
            action_unique = "toggle-recording"
            action_friendly = "Toggle Recording"

            [paste]
            mode = "fcitx5"
            command = ["ydotool", "key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"]
            fcitx5_service = "org.fcitx.Fcitx5"
            fcitx5_path = "/org/freedesktop/Fcitx5/Audiov"
            fcitx5_interface = "org.fcitx.Fcitx5.Audiov1"

            [recorder]
            backend = "pipewire"
            input_device = "alsa_input.pci-0000_00_1f.3.analog-stereo"
            "#,
        )
        .expect("write config");

        let cfg = AppConfig::load_from_path(file.path()).expect("load");

        assert!(cfg.language_detection.enabled);
        assert_eq!(cfg.language_detection.confidence_threshold, 0.70);
        assert_eq!(cfg.whisper_cpp.model, "models/ggml-large-v1.bin");
        assert_eq!(cfg.whisper_cpp.threads, 6);
        assert!(cfg.whisper_cpp.use_gpu);
        assert!(cfg.whisper_remote.enabled);
        assert_eq!(cfg.whisper_remote.api_key, "test-key");
        assert_eq!(cfg.whisper.backend, "remote");
        assert!(cfg.hotkey.enabled);
        assert_eq!(cfg.hotkey.shortcut, "Meta+H");
        assert_eq!(cfg.hotkey.action_unique, "toggle-recording");
        assert_eq!(cfg.paste.mode, "fcitx5");
        assert_eq!(cfg.paste.command[0], "ydotool");
        assert_eq!(cfg.paste.fcitx5_service, "org.fcitx.Fcitx5");
        assert_eq!(cfg.recorder.backend, "pipewire");
        assert_eq!(
            cfg.recorder.input_device.as_deref(),
            Some("alsa_input.pci-0000_00_1f.3.analog-stereo")
        );
    }
}
