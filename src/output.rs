use std::process::Command;

#[derive(Debug)]
pub enum OutputError {
    ClipboardToolUnavailable,
    ClipboardFailed,
    PasteFailed,
}

pub fn write_clipboard(text: &str) -> Result<(), OutputError> {
    let status = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        Command::new("wl-copy").arg(text).status().ok()
    } else {
        Command::new("xclip")
            .args(["-selection", "clipboard"])
            .arg(text)
            .status()
            .ok()
    };

    match status {
        Some(s) if s.success() => Ok(()),
        Some(_) => Err(OutputError::ClipboardFailed),
        None => Err(OutputError::ClipboardToolUnavailable),
    }
}

pub fn send_paste_event(command: &[String]) -> Result<(), OutputError> {
    let Some((program, args)) = command.split_first() else {
        return Err(OutputError::PasteFailed);
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
