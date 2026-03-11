use audiov::config::AppConfig;
use audiov::pipeline::SessionProcessor;
use audiov::whisper_cpp::WhisperCppEngine;

fn main() {
    let config = AppConfig::load_from_path("config.example.toml")
        .expect("failed to load config.example.toml");

    let engine = WhisperCppEngine::new(config.whisper_cpp.clone());

    let processor = SessionProcessor {
        detector: &engine,
        transcriber: &engine,
        lid_config: &config.language_detection,
    };

    // TODO: replace with real microphone capture buffer.
    let empty_audio = Vec::<i16>::new();

    match processor.process_session(&empty_audio) {
        Ok(output) => {
            println!(
                "selected_language={} reason={:?} text={}",
                output.language_decision.selected_language,
                output.language_decision.reason,
                output.text.trim()
            );
        }
        Err(err) => {
            eprintln!("session failed: {err:?}");
            std::process::exit(1);
        }
    }
}
