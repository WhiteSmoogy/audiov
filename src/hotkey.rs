use evdev::{Device, InputEventKind, Key};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum HotkeyError {
    OpenDevice(std::io::Error),
    NoMatchingDevices,
    UnsupportedKey(String),
}

pub struct HotkeyListener {
    devices: Vec<Device>,
    key: Key,
}

impl HotkeyListener {
    pub fn new(key: Key) -> Result<Self, HotkeyError> {
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
                .map(|keys| keys.contains(key))
                .unwrap_or(false)
            {
                devices.push(dev);
            }
        }

        if devices.is_empty() {
            return Err(HotkeyError::NoMatchingDevices);
        }

        Ok(Self { devices, key })
    }

    pub fn wait_for_press(&mut self) {
        self.wait_for_value(1);
    }

    pub fn wait_for_release(&mut self) {
        self.wait_for_value(0);
    }

    fn wait_for_value(&mut self, expected: i32) {
        loop {
            for dev in &mut self.devices {
                let Ok(events) = dev.fetch_events() else {
                    continue;
                };

                for event in events {
                    if let InputEventKind::Key(key) = event.kind() {
                        if key == self.key && event.value() == expected {
                            return;
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub fn parse_key(name: &str) -> Result<Key, HotkeyError> {
    match name.to_ascii_lowercase().as_str() {
        "f8" => Ok(Key::KEY_F8),
        "f9" => Ok(Key::KEY_F9),
        "f10" => Ok(Key::KEY_F10),
        "f11" => Ok(Key::KEY_F11),
        "f12" => Ok(Key::KEY_F12),
        _ => Err(HotkeyError::UnsupportedKey(name.to_owned())),
    }
}
