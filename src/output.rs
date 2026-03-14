use crate::config::PasteConfig;
use arboard::Clipboard;
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, Sender};
#[cfg(target_os = "linux")]
use std::sync::{Mutex, OnceLock};
use std::process::Command;
#[cfg(target_os = "linux")]
use std::thread;

#[derive(Debug)]
pub enum OutputError {
    ClipboardToolUnavailable,
    ClipboardFailed,
    PasteFailed,
    UnsupportedMode,
}

pub trait TextOutput {
    fn copy_to_clipboard(&self, text: &str) -> Result<(), OutputError>;
    fn trigger_paste(&self, config: &PasteConfig, text: &str) -> Result<(), OutputError>;

    fn copy_and_paste(&self, text: &str, config: &PasteConfig) -> Result<(), OutputError> {
        self.copy_to_clipboard(text)?;
        self.trigger_paste(config, text)
    }
}

#[derive(Debug, Default)]
pub struct SystemTextOutput;

impl TextOutput for SystemTextOutput {
    fn copy_to_clipboard(&self, text: &str) -> Result<(), OutputError> {
        write_clipboard(text)
    }

    fn trigger_paste(&self, config: &PasteConfig, text: &str) -> Result<(), OutputError> {
        trigger_output(config, text)
    }
}

pub fn write_clipboard(text: &str) -> Result<(), OutputError> {
    #[cfg(target_os = "linux")]
    {
        return write_clipboard_linux(text);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut clipboard = Clipboard::new().map_err(|_| OutputError::ClipboardToolUnavailable)?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| OutputError::ClipboardFailed)
    }
}

#[cfg(target_os = "linux")]
fn write_clipboard_linux(text: &str) -> Result<(), OutputError> {
    static CLIPBOARD_OWNER: OnceLock<Mutex<Option<Sender<()>>>> = OnceLock::new();

    let holder = CLIPBOARD_OWNER.get_or_init(|| Mutex::new(None));
    let text = text.to_owned();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (stop_tx, stop_rx) = mpsc::channel();

    thread::spawn(move || {
        let result = (|| {
            let mut clipboard = Clipboard::new().map_err(|_| OutputError::ClipboardToolUnavailable)?;
            clipboard
                .set_text(text)
                .map_err(|_| OutputError::ClipboardFailed)?;
            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv();
            Ok::<(), OutputError>(())
        })();

        if let Err(err) = result {
            let _ = ready_tx.send(Err(err));
        }
    });

    match ready_rx.recv() {
        Ok(Ok(())) => {
            let mut current = holder.lock().map_err(|_| OutputError::ClipboardFailed)?;
            if let Some(previous) = current.replace(stop_tx) {
                let _ = previous.send(());
            }
            Ok(())
        }
        Ok(Err(err)) => Err(err),
        Err(_) => Err(OutputError::ClipboardFailed),
    }
}

pub fn trigger_output(config: &PasteConfig, text: &str) -> Result<(), OutputError> {
    match config.mode.as_str() {
        "command" => send_paste_event(&config.command),
        "fcitx5" => commit_via_fcitx5(config, text),
        "clipboard" => Ok(()),
        _ => Err(OutputError::UnsupportedMode),
    }
}

pub fn send_paste_event(command: &[String]) -> Result<(), OutputError> {
    let Some((program, args)) = command.split_first() else {
        return Ok(());
    };

    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|_| OutputError::PasteFailed)?;

    if status.success() {
        Ok(())
    } else {
        Err(OutputError::PasteFailed)
    }
}

pub fn commit_via_fcitx5(config: &PasteConfig, text: &str) -> Result<(), OutputError> {
    let connection =
        zbus::blocking::Connection::session().map_err(|_| OutputError::PasteFailed)?;
    let proxy = zbus::blocking::Proxy::new(
        &connection,
        config.fcitx5_service.as_str(),
        config.fcitx5_path.as_str(),
        config.fcitx5_interface.as_str(),
    )
    .map_err(|_| OutputError::PasteFailed)?;
    let committed: bool = proxy
        .call("CommitText", &(text))
        .map_err(|_| OutputError::PasteFailed)?;

    if committed {
        Ok(())
    } else {
        Err(OutputError::PasteFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::write_clipboard;
    use arboard::Clipboard;

    #[test]
    #[ignore = "requires a real desktop clipboard session"]
    fn clipboard_roundtrip_smoke() {
        let marker = format!("audiov-clipboard-smoke-{}", std::process::id());
        write_clipboard(&marker).expect("write clipboard");

        let mut clipboard = Clipboard::new().expect("open clipboard");
        let current = clipboard.get_text().expect("read clipboard");
        assert_eq!(current, marker);
    }

    #[test]
    #[ignore = "requires a real desktop clipboard session"]
    fn clipboard_direct_roundtrip_smoke() {
        let marker = format!("audiov-clipboard-direct-{}", std::process::id());
        let mut clipboard = Clipboard::new().expect("open clipboard");
        clipboard.set_text(marker.clone()).expect("write clipboard");
        let current = clipboard.get_text().expect("read clipboard");
        assert_eq!(current, marker);
    }
}
