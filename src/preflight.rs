use crate::config::AppConfig;
use std::env;
use std::path::Path;

#[derive(Debug)]
pub struct PreflightWarning {
    pub message: String,
}

pub fn run_startup_checks(config: &AppConfig) -> Vec<PreflightWarning> {
    let mut warnings = Vec::new();

    if config.whisper.backend.eq_ignore_ascii_case("remote") || config.whisper_remote.enabled {
        if config.whisper_remote.api_key.trim().is_empty() {
            warnings.push(PreflightWarning {
                message: "whisper remote enabled but whisper_remote.api_key is empty".to_owned(),
            });
        }
    }

    if !(config.whisper.backend.eq_ignore_ascii_case("remote") || config.whisper_remote.enabled)
        && !Path::new(&config.whisper_cpp.model).exists()
    {
        warnings.push(PreflightWarning {
            message: format!(
                "whisper model file not found: {} (update [whisper_cpp].model)",
                config.whisper_cpp.model
            ),
        });
    }

    if !tool_exists("arecord") {
        warnings.push(PreflightWarning {
            message: "missing recorder dependency: arecord".to_owned(),
        });
    }

    let clipboard_program = if env::var("WAYLAND_DISPLAY").is_ok() {
        "wl-copy"
    } else {
        "xclip"
    };

    if !tool_exists(clipboard_program) {
        warnings.push(PreflightWarning {
            message: format!("missing clipboard dependency: {clipboard_program}"),
        });
    }

    if let Some(program) = config.paste.command.first() {
        if !tool_exists(program) {
            warnings.push(PreflightWarning {
                message: format!("missing paste command program: {program}"),
            });
        }
    } else {
        warnings.push(PreflightWarning {
            message: "paste.command is empty; paste injection will fail".to_owned(),
        });
    }

    warnings
}

fn tool_exists(program: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(program);
        candidate.exists() && candidate.is_file()
    })
}
