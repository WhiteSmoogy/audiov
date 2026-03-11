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
}

pub struct ActiveRecording {
    child: Child,
    file_path: PathBuf,
}

pub struct ArecordRecorder;

impl ArecordRecorder {
    pub fn start() -> Result<ActiveRecording, RecorderError> {
        let file_path = std::env::temp_dir().join(format!("audiov-{}.wav", std::process::id()));
        let child = Command::new("arecord")
            .args(["-q", "-f", "S16_LE", "-r", "16000", "-c", "1", "-t", "wav"])
            .arg(&file_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(RecorderError::Spawn)?;

        Ok(ActiveRecording { child, file_path })
    }
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
