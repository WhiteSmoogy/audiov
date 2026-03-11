use crate::config::RecorderConfig;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[derive(Debug)]
pub enum RecorderError {
    Spawn(std::io::Error),
    Signal(nix::Error),
    Wait(std::io::Error),
    Wav(hound::Error),
    UnsupportedBackend(String),
    BackendUnavailable(String),
}

pub struct ActiveRecording {
    child: Child,
    file_path: PathBuf,
}

pub struct NativeRecorder {
    backend: RecorderBackend,
    input_device: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum RecorderBackend {
    Auto,
    PipeWire,
    PulseAudio,
    Alsa,
}

impl NativeRecorder {
    pub fn from_config(config: &RecorderConfig) -> Result<Self, RecorderError> {
        let backend = parse_backend(&config.backend)?;
        Ok(Self {
            backend,
            input_device: config.input_device.clone(),
        })
    }

    pub fn start(&self) -> Result<ActiveRecording, RecorderError> {
        let file_path = std::env::temp_dir().join(format!("audiov-{}.wav", std::process::id()));

        let mut candidates: Vec<Vec<String>> = Vec::new();
        match self.backend {
            RecorderBackend::Auto => {
                candidates.push(build_pw_cat_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
                candidates.push(build_parec_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
                candidates.push(build_arecord_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
            }
            RecorderBackend::PipeWire => {
                candidates.push(build_pw_cat_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
            }
            RecorderBackend::PulseAudio => {
                candidates.push(build_parec_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
            }
            RecorderBackend::Alsa => {
                candidates.push(build_arecord_command(
                    &file_path,
                    self.input_device.as_deref(),
                ));
            }
        }

        let mut last_spawn_error = None;
        for args in candidates {
            let executable = &args[0];
            let spawn_result = Command::new(executable)
                .args(&args[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            match spawn_result {
                Ok(child) => {
                    return Ok(ActiveRecording { child, file_path });
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    last_spawn_error = Some(err);
                }
                Err(err) => return Err(RecorderError::Spawn(err)),
            }
        }

        Err(RecorderError::BackendUnavailable(
            last_spawn_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "no recorder backend available".to_owned()),
        ))
    }
}

fn parse_backend(raw: &str) -> Result<RecorderBackend, RecorderError> {
    match raw.to_ascii_lowercase().as_str() {
        "auto" => Ok(RecorderBackend::Auto),
        "pipewire" | "pw" => Ok(RecorderBackend::PipeWire),
        "pulseaudio" | "pulse" => Ok(RecorderBackend::PulseAudio),
        "alsa" | "arecord" => Ok(RecorderBackend::Alsa),
        other => Err(RecorderError::UnsupportedBackend(other.to_owned())),
    }
}

fn build_pw_cat_command(file_path: &PathBuf, input_device: Option<&str>) -> Vec<String> {
    let mut cmd = vec!["pw-cat".to_owned(), "--record".to_owned()];
    if let Some(device) = input_device {
        cmd.push("--target".to_owned());
        cmd.push(device.to_owned());
    }
    cmd.push("--rate".to_owned());
    cmd.push("16000".to_owned());
    cmd.push("--channels".to_owned());
    cmd.push("1".to_owned());
    cmd.push("--format".to_owned());
    cmd.push("s16".to_owned());
    cmd.push(file_path.display().to_string());
    cmd
}

fn build_parec_command(file_path: &PathBuf, input_device: Option<&str>) -> Vec<String> {
    let mut cmd = vec![
        "parec".to_owned(),
        "--format=s16le".to_owned(),
        "--rate=16000".to_owned(),
        "--channels=1".to_owned(),
        "--file-format=wav".to_owned(),
    ];
    if let Some(device) = input_device {
        cmd.push("--device".to_owned());
        cmd.push(device.to_owned());
    }
    cmd.push(file_path.display().to_string());
    cmd
}

fn build_arecord_command(file_path: &PathBuf, input_device: Option<&str>) -> Vec<String> {
    let mut cmd = vec![
        "arecord".to_owned(),
        "-q".to_owned(),
        "-f".to_owned(),
        "S16_LE".to_owned(),
        "-r".to_owned(),
        "16000".to_owned(),
        "-c".to_owned(),
        "1".to_owned(),
        "-t".to_owned(),
        "wav".to_owned(),
    ];
    if let Some(device) = input_device {
        cmd.push("-D".to_owned());
        cmd.push(device.to_owned());
    }
    cmd.push(file_path.display().to_string());
    cmd
}

impl ActiveRecording {
    pub fn stop_and_collect(mut self) -> Result<Vec<i16>, RecorderError> {
        let pid = Pid::from_raw(self.child.id() as i32);
        kill(pid, Signal::SIGINT).map_err(RecorderError::Signal)?;
        self.child.wait().map_err(RecorderError::Wait)?;

        let mut reader = hound::WavReader::open(&self.file_path).map_err(RecorderError::Wav)?;
        let samples = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(RecorderError::Wav)?;

        let _ = std::fs::remove_file(&self.file_path);
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_backend_accepts_native_audio_backends() {
        assert!(matches!(parse_backend("auto"), Ok(RecorderBackend::Auto)));
        assert!(matches!(
            parse_backend("pipewire"),
            Ok(RecorderBackend::PipeWire)
        ));
        assert!(matches!(
            parse_backend("pulse"),
            Ok(RecorderBackend::PulseAudio)
        ));
        assert!(matches!(parse_backend("alsa"), Ok(RecorderBackend::Alsa)));
    }

    #[test]
    fn parse_backend_rejects_unknown_values() {
        assert!(matches!(
            parse_backend("jack"),
            Err(RecorderError::UnsupportedBackend(v)) if v == "jack"
        ));
    }
}
