use super::*;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(super) struct SaveData {
    pub(super) config: GameConfig,
    pub(super) high_score: u32,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            config: GameConfig::default(),
            high_score: 0,
        }
    }
}

impl SaveData {
    fn encode(self) -> String {
        format!("{}\nhigh_score={}", self.config.encode(), self.high_score)
    }

    fn decode(raw: &str) -> Self {
        let mut save = Self::default();
        if let Some(config) = GameConfig::decode(raw) {
            save.config = config;
        }
        for line in raw.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() == "high_score" {
                if let Ok(score) = value.trim().parse() {
                    save.high_score = score;
                }
            }
        }
        save
    }
}

pub(super) fn load_save_data() -> SaveData {
    load_save_blob()
        .map(|raw| SaveData::decode(&raw))
        .unwrap_or_default()
}

pub(super) fn store_save_data(save: SaveData) {
    store_save_blob(&save.encode());
}

#[cfg(target_arch = "wasm32")]
fn load_save_blob() -> Option<String> {
    let len = unsafe { mcommand_storage_get_len() };
    if len <= 0 {
        return None;
    }

    let mut buffer = vec![0u8; len as usize];
    unsafe {
        mcommand_storage_get(buffer.as_mut_ptr(), len as u32);
    }
    String::from_utf8(buffer).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn load_save_blob() -> Option<String> {
    std::fs::read_to_string(native_save_path()).ok()
}

#[cfg(target_arch = "wasm32")]
fn store_save_blob(raw: &str) {
    unsafe {
        mcommand_storage_set(raw.as_ptr(), raw.len() as u32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn store_save_blob(raw: &str) {
    let _ = std::fs::write(native_save_path(), raw);
}

#[cfg(not(target_arch = "wasm32"))]
fn native_save_path() -> PathBuf {
    let mut path = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push(".mcommand-save");
    path
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn mcommand_storage_get_len() -> i32;
    fn mcommand_storage_get(ptr: *mut u8, len: u32);
    fn mcommand_storage_set(ptr: *const u8, len: u32);
}
