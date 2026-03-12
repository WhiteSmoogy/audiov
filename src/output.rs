use arboard::Clipboard;
use std::process::Command;

#[derive(Debug)]
pub enum OutputError {
    ClipboardToolUnavailable,
    ClipboardFailed,
    PasteFailed,
}

pub trait TextOutput {
    fn copy_to_clipboard(&self, text: &str) -> Result<(), OutputError>;
    fn trigger_paste(&self, command: &[String]) -> Result<(), OutputError>;

    fn copy_and_paste(&self, text: &str, command: &[String]) -> Result<(), OutputError> {
        self.copy_to_clipboard(text)?;

        if command.is_empty() {
            return Ok(());
        }

        self.trigger_paste(command)
    }
}

#[derive(Debug, Default)]
pub struct SystemTextOutput;

impl TextOutput for SystemTextOutput {
    fn copy_to_clipboard(&self, text: &str) -> Result<(), OutputError> {
        write_clipboard(text)
    }

    fn trigger_paste(&self, command: &[String]) -> Result<(), OutputError> {
        send_paste_event(command)
    }
}

pub fn write_clipboard(text: &str) -> Result<(), OutputError> {
    let mut clipboard = Clipboard::new().map_err(|_| OutputError::ClipboardToolUnavailable)?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|_| OutputError::ClipboardFailed)
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
