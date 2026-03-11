use audiov::config::AppConfig;
use audiov::hotkey::{parse_key, HotkeyListener};
use audiov::output::{send_paste_event, write_clipboard};
use audiov::pipeline::SessionProcessor;
use audiov::recorder::NativeRecorder;
use audiov::whisper_cpp::WhisperCppEngine;

fn main() {
    let config = AppConfig::load_from_path("config.example.toml")
        .expect("failed to load config.example.toml");

    let key = parse_key(&config.hotkey.key).expect("invalid hotkey key");
    let mut listener = HotkeyListener::new(key).expect("failed to init hotkey listener");

    let engine = WhisperCppEngine::new(config.whisper_cpp.clone());
    let recorder =
        NativeRecorder::from_config(&config.recorder).expect("failed to init recorder backend");
    let processor = SessionProcessor {
        detector: &engine,
        transcriber: &engine,
        lid_config: &config.language_detection,
    };

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
