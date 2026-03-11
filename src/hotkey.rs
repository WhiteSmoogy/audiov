use evdev::{Device, InputEventKind, Key};
use std::collections::HashSet;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum HotkeyError {
    OpenDevice(std::io::Error),
    NoMatchingDevices,
    UnsupportedKey(String),
    EmptyHotkey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HotkeyBinding {
    pub keys: Vec<Key>,
}

pub struct HotkeyListener {
    devices: Vec<Device>,
    keys: Vec<Key>,
    pressed: HashSet<Key>,
}

impl HotkeyListener {
    pub fn new(binding: HotkeyBinding) -> Result<Self, HotkeyError> {
        let mut devices = Vec::new();
        let entries =
            std::fs::read_dir(Path::new("/dev/input")).map_err(HotkeyError::OpenDevice)?;

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };
            if !name.starts_with("event") {
                continue;
            }

            let Ok(dev) = Device::open(&path) else {
                continue;
            };

            if dev
                .supported_keys()
                .map(|supported| binding.keys.iter().any(|k| supported.contains(*k)))
                .unwrap_or(false)
            {
                devices.push(dev);
            }
        }

        if devices.is_empty() {
            return Err(HotkeyError::NoMatchingDevices);
        }

        Ok(Self {
            devices,
            keys: binding.keys,
            pressed: HashSet::new(),
        })
    }

    pub fn wait_for_press(&mut self) {
        self.wait_until_all_pressed();
    }

    pub fn wait_for_release(&mut self) {
        loop {
            self.poll_events();
            if self.keys.iter().all(|k| !self.pressed.contains(k)) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_until_all_pressed(&mut self) {
        loop {
            self.poll_events();
            if self.keys.iter().all(|k| self.pressed.contains(k)) {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn poll_events(&mut self) {
        for dev in &mut self.devices {
            let Ok(events) = dev.fetch_events() else {
                continue;
            };

            for event in events {
                if let InputEventKind::Key(key) = event.kind() {
                    if self.keys.contains(&key) {
                        if event.value() == 0 {
                            self.pressed.remove(&key);
                        } else {
                            self.pressed.insert(key);
                        }
                    }
                }
            }
        }
    }
}

pub fn parse_hotkey(hotkey: &str) -> Result<HotkeyBinding, HotkeyError> {
    let mut keys = Vec::new();

    for part in hotkey.split('+') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        keys.push(parse_key(trimmed)?);
    }

    if keys.is_empty() {
        return Err(HotkeyError::EmptyHotkey);
    }

    let mut dedup = Vec::new();
    for key in keys {
        if !dedup.contains(&key) {
            dedup.push(key);
        }
    }

    Ok(HotkeyBinding { keys: dedup })
}

fn parse_key(name: &str) -> Result<Key, HotkeyError> {
    let normalized = name.to_ascii_lowercase();
    let key = normalized
        .strip_prefix("key_")
        .unwrap_or(normalized.as_str())
        .to_owned();

    if let Some(rest) = key.strip_prefix('f') {
        if let Ok(n) = rest.parse::<u8>() {
            return function_key(n).ok_or_else(|| HotkeyError::UnsupportedKey(name.to_owned()));
        }
    }

    let parsed = match key.as_str() {
        "esc" | "escape" => Some(Key::KEY_ESC),
        "enter" | "return" => Some(Key::KEY_ENTER),
        "space" => Some(Key::KEY_SPACE),
        "tab" => Some(Key::KEY_TAB),
        "leftctrl" | "lctrl" | "ctrl" => Some(Key::KEY_LEFTCTRL),
        "rightctrl" | "rctrl" => Some(Key::KEY_RIGHTCTRL),
        "leftshift" | "lshift" | "shift" => Some(Key::KEY_LEFTSHIFT),
        "rightshift" | "rshift" => Some(Key::KEY_RIGHTSHIFT),
        "leftalt" | "lalt" | "alt" => Some(Key::KEY_LEFTALT),
        "rightalt" | "ralt" => Some(Key::KEY_RIGHTALT),
        "leftmeta" | "lmeta" | "meta" | "super" | "win" | "windows" => Some(Key::KEY_LEFTMETA),
        "rightmeta" | "rmeta" => Some(Key::KEY_RIGHTMETA),
        _ => alpha_numeric_key(&key),
    };

    parsed.ok_or_else(|| HotkeyError::UnsupportedKey(name.to_owned()))
}

fn function_key(n: u8) -> Option<Key> {
    match n {
        1 => Some(Key::KEY_F1),
        2 => Some(Key::KEY_F2),
        3 => Some(Key::KEY_F3),
        4 => Some(Key::KEY_F4),
        5 => Some(Key::KEY_F5),
        6 => Some(Key::KEY_F6),
        7 => Some(Key::KEY_F7),
        8 => Some(Key::KEY_F8),
        9 => Some(Key::KEY_F9),
        10 => Some(Key::KEY_F10),
        11 => Some(Key::KEY_F11),
        12 => Some(Key::KEY_F12),
        13 => Some(Key::KEY_F13),
        14 => Some(Key::KEY_F14),
        15 => Some(Key::KEY_F15),
        16 => Some(Key::KEY_F16),
        17 => Some(Key::KEY_F17),
        18 => Some(Key::KEY_F18),
        19 => Some(Key::KEY_F19),
        20 => Some(Key::KEY_F20),
        21 => Some(Key::KEY_F21),
        22 => Some(Key::KEY_F22),
        23 => Some(Key::KEY_F23),
        24 => Some(Key::KEY_F24),
        _ => None,
    }
}

fn alpha_numeric_key(name: &str) -> Option<Key> {
    match name {
        "a" => Some(Key::KEY_A),
        "b" => Some(Key::KEY_B),
        "c" => Some(Key::KEY_C),
        "d" => Some(Key::KEY_D),
        "e" => Some(Key::KEY_E),
        "f" => Some(Key::KEY_F),
        "g" => Some(Key::KEY_G),
        "h" => Some(Key::KEY_H),
        "i" => Some(Key::KEY_I),
        "j" => Some(Key::KEY_J),
        "k" => Some(Key::KEY_K),
        "l" => Some(Key::KEY_L),
        "m" => Some(Key::KEY_M),
        "n" => Some(Key::KEY_N),
        "o" => Some(Key::KEY_O),
        "p" => Some(Key::KEY_P),
        "q" => Some(Key::KEY_Q),
        "r" => Some(Key::KEY_R),
        "s" => Some(Key::KEY_S),
        "t" => Some(Key::KEY_T),
        "u" => Some(Key::KEY_U),
        "v" => Some(Key::KEY_V),
        "w" => Some(Key::KEY_W),
        "x" => Some(Key::KEY_X),
        "y" => Some(Key::KEY_Y),
        "z" => Some(Key::KEY_Z),
        "0" => Some(Key::KEY_0),
        "1" => Some(Key::KEY_1),
        "2" => Some(Key::KEY_2),
        "3" => Some(Key::KEY_3),
        "4" => Some(Key::KEY_4),
        "5" => Some(Key::KEY_5),
        "6" => Some(Key::KEY_6),
        "7" => Some(Key::KEY_7),
        "8" => Some(Key::KEY_8),
        "9" => Some(Key::KEY_9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_key() {
        let binding = parse_hotkey("F12").unwrap();
        assert_eq!(binding.keys, vec![Key::KEY_F12]);
    }

    #[test]
    fn parses_windows_h_combo() {
        let binding = parse_hotkey("windows+h").unwrap();
        assert_eq!(binding.keys.len(), 2);
        assert!(binding.keys.contains(&Key::KEY_LEFTMETA));
        assert!(binding.keys.contains(&Key::KEY_H));
    }

    #[test]
    fn parses_combo_with_spaces_and_aliases() {
        let binding = parse_hotkey(" win + h ").unwrap();
        assert_eq!(binding.keys.len(), 2);
        assert!(binding.keys.contains(&Key::KEY_LEFTMETA));
        assert!(binding.keys.contains(&Key::KEY_H));
    }

    #[test]
    fn rejects_unknown_or_empty() {
        assert!(parse_hotkey("").is_err());
        assert!(parse_hotkey("+").is_err());
        assert!(parse_hotkey("windows+hyper").is_err());
    }
}
