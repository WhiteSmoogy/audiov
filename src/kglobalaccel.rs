use crate::config::HotkeyConfig;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

const SERVICE_NAME: &str = "org.kde.kglobalaccel";
const ROOT_PATH: &str = "/kglobalaccel";
const ROOT_INTERFACE: &str = "org.kde.KGlobalAccel";
const COMPONENT_INTERFACE: &str = "org.kde.kglobalaccel.Component";
const SET_PRESENT: u32 = 0x2;
const NO_AUTOLOADING: u32 = 0x4;
const QT_SHIFT: i32 = 0x0200_0000;
const QT_CTRL: i32 = 0x0400_0000;
const QT_ALT: i32 = 0x0800_0000;
const QT_META: i32 = 0x1000_0000;

#[derive(Debug)]
pub enum KGlobalAccelError {
    Dbus(zbus::Error),
    InvalidShortcut(String),
}

impl From<zbus::Error> for KGlobalAccelError {
    fn from(value: zbus::Error) -> Self {
        Self::Dbus(value)
    }
}

pub struct KGlobalAccelListener {
    connection: Connection,
    component_path: String,
    component_unique: String,
    action_unique: String,
    action_id: Vec<String>,
}

impl KGlobalAccelListener {
    pub fn new(config: &HotkeyConfig) -> Result<Self, KGlobalAccelError> {
        let connection = Connection::session()?;
        let root = Proxy::new(&connection, SERVICE_NAME, ROOT_PATH, ROOT_INTERFACE)?;

        let action_id = vec![
            config.component_unique.clone(),
            config.action_unique.clone(),
            config.component_friendly.clone(),
            config.action_friendly.clone(),
        ];

        root.call_noreply("doRegister", &(action_id.clone(),))?;

        let keys = parse_shortcut(&config.shortcut)?;
        set_shortcut(&root, &action_id, &keys)?;

        let path: OwnedObjectPath = root.call("getComponent", &(config.component_unique.as_str(),))?;

        Ok(Self {
            connection,
            component_path: path.to_string(),
            component_unique: config.component_unique.clone(),
            action_unique: config.action_unique.clone(),
            action_id,
        })
    }

    pub fn wait_for_trigger(&self) -> Result<(), KGlobalAccelError> {
        let component = Proxy::new(
            &self.connection,
            SERVICE_NAME,
            self.component_path.as_str(),
            COMPONENT_INTERFACE,
        )?;
        let mut pressed = component.receive_signal_with_args(
            "globalShortcutPressed",
            &[
                (0, self.component_unique.as_str()),
                (1, self.action_unique.as_str()),
            ],
        )?;

        let Some(message) = pressed.next() else {
            return Err(KGlobalAccelError::InvalidShortcut(
                "kglobalaccel signal stream ended unexpectedly".to_owned(),
            ));
        };
        let _: (String, String, i64) = message.body().deserialize()?;
        Ok(())
    }
}

fn set_shortcut(
    root: &Proxy<'_>,
    action_id: &[String],
    keys: &[i32],
) -> Result<Vec<i32>, KGlobalAccelError> {
    Ok(root.call(
        "setShortcut",
        &(action_id.to_vec(), keys.to_vec(), SET_PRESENT | NO_AUTOLOADING),
    )?)
}

impl Drop for KGlobalAccelListener {
    fn drop(&mut self) {
        let root = Proxy::new(&self.connection, SERVICE_NAME, ROOT_PATH, ROOT_INTERFACE);
        if let Ok(root) = root {
            let _ = root.call_noreply("setInactive", &(self.action_id.clone(),));
        }
    }
}

fn parse_shortcut(raw: &str) -> Result<Vec<i32>, KGlobalAccelError> {
    let mut modifiers = 0;
    let mut key = None;

    for token in raw.split('+').map(str::trim).filter(|token| !token.is_empty()) {
        match token.to_ascii_lowercase().as_str() {
            "shift" => modifiers |= QT_SHIFT,
            "ctrl" | "control" => modifiers |= QT_CTRL,
            "alt" => modifiers |= QT_ALT,
            "meta" | "super" | "win" | "windows" => modifiers |= QT_META,
            other => {
                if key.is_some() {
                    return Err(KGlobalAccelError::InvalidShortcut(raw.to_owned()));
                }
                key = Some(parse_key(other).ok_or_else(|| {
                    KGlobalAccelError::InvalidShortcut(raw.to_owned())
                })?);
            }
        }
    }

    let key = key.ok_or_else(|| KGlobalAccelError::InvalidShortcut(raw.to_owned()))?;
    Ok(vec![modifiers | key])
}

fn parse_key(raw: &str) -> Option<i32> {
    if raw.len() == 1 {
        let ch = raw.chars().next()?;
        return Some(match ch {
            'a'..='z' => ch.to_ascii_uppercase() as i32,
            'A'..='Z' => ch as i32,
            '0'..='9' => ch as i32,
            _ => return None,
        });
    }

    match raw {
        "space" => Some(0x20),
        "tab" => Some(0x0100_0001),
        "escape" | "esc" => Some(0x0100_0000),
        "return" => Some(0x0100_0004),
        "enter" => Some(0x0100_0005),
        "backspace" => Some(0x0100_0003),
        _ => raw
            .strip_prefix('f')
            .and_then(|suffix| suffix.parse::<i32>().ok())
            .filter(|value| (1..=35).contains(value))
            .map(|value| 0x0100_0030 + value - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meta_h() {
        assert_eq!(parse_shortcut("Meta+H").unwrap(), vec![0x1000_0048]);
    }

    #[test]
    fn parses_ctrl_shift_f5() {
        assert_eq!(
            parse_shortcut("Ctrl+Shift+F5").unwrap(),
            vec![0x0700_0034]
        );
    }

    #[test]
    fn rejects_missing_key() {
        assert!(matches!(
            parse_shortcut("Meta+Shift"),
            Err(KGlobalAccelError::InvalidShortcut(_))
        ));
    }
}
