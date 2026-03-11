use audiov::config::AppConfig;
use audiov::hotkey::{parse_hotkey, HotkeyListener};
use audiov::output::{send_paste_event, write_clipboard};
use audiov::pipeline::SessionProcessor;
use audiov::preflight::run_startup_checks;
use audiov::recorder::NativeRecorder;
use audiov::whisper_cpp::WhisperCppEngine;
use std::env;
use std::process::{Command, Stdio};

#[derive(Debug)]
struct CliArgs {
    config_path: String,
    daemon: bool,
    foreground: bool,
}

fn main() {
    let args = parse_args();

    if args.daemon && !args.foreground {
        spawn_daemon(&args);
        return;
    }

    let config = AppConfig::load_from_path(&args.config_path)
        .unwrap_or_else(|_| panic!("failed to load config: {}", args.config_path));

    for warning in run_startup_checks(&config) {
        eprintln!("[WARN] {}", warning.message);
    }

    let binding = parse_hotkey(&config.hotkey.key).expect("invalid hotkey key");
    let mut listener = HotkeyListener::new(binding).expect("failed to init hotkey listener");

    let engine = WhisperCppEngine::new(config.whisper_cpp.clone());
    let recorder =
        NativeRecorder::from_config(&config.recorder).expect("failed to init recorder backend");
    let processor = SessionProcessor {
        detector: &engine,
        transcriber: &engine,
        lid_config: &config.language_detection,
    };

    eprintln!("[INFO] audiov started, hotkey={}", config.hotkey.key);

    loop {
        listener.wait_for_press();

        let recording = match recorder.start() {
            Ok(r) => r,
            Err(err) => {
                eprintln!("recording start failed: {err:?}");
                continue;
            }
        };

        listener.wait_for_release();

        let audio = match recording.stop_and_collect() {
            Ok(samples) => samples,
            Err(err) => {
                eprintln!("recording stop failed: {err:?}");
                continue;
            }
        };

        if audio.is_empty() {
            continue;
        }

        let output = match processor.process_session(&audio) {
            Ok(out) => out,
            Err(err) => {
                eprintln!("transcription failed: {err:?}");
                continue;
            }
        };

        eprintln!(
            "[DEBUG] language={} reason={:?}",
            output.language_decision.selected_language, output.language_decision.reason
        );

        if output.text.trim().is_empty() {
            continue;
        }

        if let Err(err) = write_clipboard(output.text.trim()) {
            eprintln!("clipboard failed: {err:?}");
            continue;
        }

        if let Err(err) = send_paste_event(&config.paste.command) {
            eprintln!("paste event failed: {err:?}");
        }
    }
}

fn parse_args() -> CliArgs {
    let mut config_path: Option<String> = None;
    let mut daemon = false;
    let mut foreground = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--daemon" => daemon = true,
            "--foreground" => foreground = true,
            "--config" => {
                if let Some(path) = args.next() {
                    config_path = Some(path);
                }
            }
            _ => {}
        }
    }

    CliArgs {
        config_path: config_path.unwrap_or_else(default_config_path),
        daemon,
        foreground,
    }
}

fn default_config_path() -> String {
    if let Ok(path) = env::var("AUDIOV_CONFIG") {
        return path;
    }

    if let Ok(home) = env::var("HOME") {
        let user_path = format!("{home}/.config/audiov/config.toml");
        if std::path::Path::new(&user_path).exists() {
            return user_path;
        }
    }

    panic!("no config found: pass --config or set AUDIOV_CONFIG (default ~/.config/audiov/config.toml)");
}

fn spawn_daemon(args: &CliArgs) {
    let exe = env::current_exe().expect("resolve current executable failed");
    let mut command = Command::new(exe);
    command
        .arg("--foreground")
        .arg("--config")
        .arg(&args.config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match command.spawn() {
        Ok(_) => eprintln!("[INFO] audiov daemonized"),
        Err(err) => panic!("failed to daemonize: {err}"),
    }
}
