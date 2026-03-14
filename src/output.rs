use crate::config::PasteConfig;

#[derive(Debug)]
pub enum OutputError {
    PasteFailed,
}

pub trait TextOutput {
    fn trigger_paste(&self, config: &PasteConfig, text: &str) -> Result<(), OutputError>;

    fn copy_and_paste(&self, text: &str, config: &PasteConfig) -> Result<(), OutputError> {
        self.trigger_paste(config, text)
    }
}

#[derive(Debug, Default)]
pub struct SystemTextOutput;

impl TextOutput for SystemTextOutput {
    fn trigger_paste(&self, config: &PasteConfig, text: &str) -> Result<(), OutputError> {
        commit_via_fcitx5(config, text)
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
