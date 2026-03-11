use audiov::config::AppConfig;
use audiov::output::{send_paste_event, write_clipboard};
use audiov::pipeline::{LanguageDetector, SessionProcessor, WhisperTranscriber};
use audiov::preflight::run_startup_checks;
use audiov::recorder::NativeRecorder;
use audiov::whisper_cpp::WhisperCppEngine;
use audiov::whisper_remote::WhisperRemoteEngine;
use std::env;
use std::io::{self, IsTerminal, Write};

#[derive(Debug)]
struct CliArgs {
    config_path: String,
    daemon: bool,
    foreground: bool,
    transcribe_wav: Option<String>,
}

enum ActiveEngine {
    Cpp(WhisperCppEngine),
    Remote(WhisperRemoteEngine),
}

impl LanguageDetector for ActiveEngine {
    fn detect_language(&self, pcm_s16le_mono_16k: &[i16]) -> Vec<audiov::lid::DetectionCandidate> {
        match self {
            ActiveEngine::Cpp(engine) => engine.detect_language(pcm_s16le_mono_16k),
            ActiveEngine::Remote(engine) => engine.detect_language(pcm_s16le_mono_16k),
        }
    }
}

impl WhisperTranscriber for ActiveEngine {
    fn transcribe(
        &self,
        pcm_s16le_mono_16k: &[i16],
        language: Option<&str>,
    ) -> Result<String, audiov::pipeline::TranscriptionError> {
        match self {
            ActiveEngine::Cpp(engine) => engine.transcribe(pcm_s16le_mono_16k, language),
            ActiveEngine::Remote(engine) => engine.transcribe(pcm_s16le_mono_16k, language),
        }
    }
}

fn main() {
    let args = parse_args();

    if let Some(wav_path) = args.transcribe_wav.as_deref() {
        run_wav_transcription(&args.config_path, wav_path);
        return;
    }

    reject_daemon_mode(&args);

    let config = AppConfig::load_from_path(&args.config_path)
        .unwrap_or_else(|_| panic!("failed to load config: {}", args.config_path));

    for warning in run_startup_checks(&config) {
        eprintln!("[WARN] {}", warning.message);
    }

    let engine =
        if config.whisper.backend.eq_ignore_ascii_case("remote") || config.whisper_remote.enabled {
            ActiveEngine::Remote(
                WhisperRemoteEngine::new(config.whisper_remote.clone())
                    .expect("failed to init whisper remote engine"),
            )
        } else {
            ActiveEngine::Cpp(
                WhisperCppEngine::new(config.whisper_cpp.clone())
                    .expect("failed to init whisper cpp engine"),
            )
        };

    let recorder =
        NativeRecorder::from_config(&config.recorder).expect("failed to init recorder backend");
    let processor = SessionProcessor {
        detector: &engine,
        transcriber: &engine,
        lid_config: &config.language_detection,
    };

    run_manual_loop(&recorder, &processor, &config);
}

fn parse_args() -> CliArgs {
    let mut config_path: Option<String> = None;
    let mut daemon = false;
    let mut foreground = false;
    let mut _manual = false;
    let mut transcribe_wav: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--daemon" => daemon = true,
            "--foreground" => foreground = true,
            "--manual" => _manual = true,
            "--transcribe-wav" => {
                if let Some(path) = args.next() {
                    transcribe_wav = Some(path);
                }
            }
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
        transcribe_wav,
    }
}

fn run_wav_transcription(config_path: &str, wav_path: &str) {
    let config = AppConfig::load_from_path(config_path)
        .unwrap_or_else(|_| panic!("failed to load config: {config_path}"));

    for warning in run_startup_checks(&config) {
        eprintln!("[WARN] {}", warning.message);
    }

    let engine =
        if config.whisper.backend.eq_ignore_ascii_case("remote") || config.whisper_remote.enabled {
            ActiveEngine::Remote(
                WhisperRemoteEngine::new(config.whisper_remote.clone())
                    .expect("failed to init whisper remote engine"),
            )
        } else {
            ActiveEngine::Cpp(
                WhisperCppEngine::new(config.whisper_cpp.clone())
                    .expect("failed to init whisper cpp engine"),
            )
        };

    let audio = read_wav_mono_16k_i16(wav_path);
    let processor = SessionProcessor {
        detector: &engine,
        transcriber: &engine,
        lid_config: &config.language_detection,
    };

    let output = processor
        .process_session(&audio)
        .unwrap_or_else(|err| panic!("transcription failed: {err:?}"));

    eprintln!(
        "[DEBUG] language={} reason={:?}",
        output.language_decision.selected_language, output.language_decision.reason
    );
    println!("{}", output.text.trim());
}

fn read_wav_mono_16k_i16(path: &str) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path)
        .unwrap_or_else(|err| panic!("failed to open wav file {path}: {err}"));
    let spec = reader.spec();
    assert_eq!(
        spec.channels, 1,
        "wav must be mono: expected 1 channel, got {}",
        spec.channels
    );
    assert_eq!(
        spec.sample_rate, 16_000,
        "wav must be 16kHz: expected 16000, got {}",
        spec.sample_rate
    );
    assert_eq!(
        spec.bits_per_sample, 16,
        "wav must be 16-bit PCM: expected 16 bits, got {}",
        spec.bits_per_sample
    );

    reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| panic!("failed to read wav samples from {path}: {err}"))
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

fn handle_transcription_result<D, T>(
    processor: &SessionProcessor<'_, D, T>,
    config: &AppConfig,
    audio: &[i16],
) where
    D: LanguageDetector,
    T: WhisperTranscriber,
{
    let output = match processor.process_session(audio) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("transcription failed: {err:?}");
            return;
        }
    };

    let text = output.text.trim();
    eprintln!(
        "[DEBUG] language={} reason={:?}",
        output.language_decision.selected_language, output.language_decision.reason
    );

    if text.is_empty() {
        eprintln!("[INFO] empty transcription");
        return;
    }

    println!("{text}");

    if let Err(err) = write_clipboard(text) {
        eprintln!("[WARN] clipboard failed: {err:?}");
        return;
    }

    if config.paste.command.is_empty() {
        return;
    }

    if let Err(err) = send_paste_event(&config.paste.command) {
        eprintln!("[WARN] paste event failed: {err:?}");
    }
}

fn run_manual_loop<D, T>(
    recorder: &NativeRecorder,
    processor: &SessionProcessor<'_, D, T>,
    config: &AppConfig,
) where
    D: LanguageDetector,
    T: WhisperTranscriber,
{
    if !io::stdin().is_terminal() {
        panic!("manual mode requires an interactive terminal");
    }

    eprintln!("[INFO] audiov started in manual mode");
    eprintln!("[INFO] press Enter to start recording, Enter again to stop, Ctrl+C to exit");

    loop {
        wait_for_enter("start");
        let recording = match recorder.start() {
            Ok(r) => r,
            Err(err) => {
                eprintln!("recording start failed: {err:?}");
                continue;
            }
        };
        eprintln!("[INFO] recording...");

        wait_for_enter("stop");
        let audio = match recording.stop_and_collect() {
            Ok(samples) => samples,
            Err(err) => {
                eprintln!("recording stop failed: {err:?}");
                continue;
            }
        };

        if audio.is_empty() {
            eprintln!("[INFO] empty recording");
            continue;
        }

        handle_transcription_result(processor, config, &audio);
    }
}

fn wait_for_enter(action: &str) {
    print!("Press Enter to {action}...");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

fn reject_daemon_mode(args: &CliArgs) {
    if args.daemon && !args.foreground {
        panic!("--daemon is not supported after hotkey mode removal; use --manual in a terminal");
    }
}
