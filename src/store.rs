use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::mind_map::MindMap;

static DIR_NAME: &str = env!("CARGO_PKG_NAME");
static DATA_FILE: &str = "data.json";

#[derive(Serialize, Deserialize)]
struct DataWrapper {
    maps: Vec<MindMap>,
    active_index: usize,
}

/// Borrowed view for serialization — avoids cloning all maps on every save.
#[derive(Serialize)]
struct DataWrapperRef<'a> {
    maps: &'a [MindMap],
    active_index: usize,
}

pub struct Store;

impl Store {
    fn dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(DIR_NAME);
        path
    }

    fn data_path() -> PathBuf {
        let mut path = Self::dir();
        path.push(DATA_FILE);
        path
    }

    pub fn load() -> (Vec<MindMap>, usize) {
        fs::create_dir_all(Self::dir()).ok();
        let path = Self::data_path();
        if path.exists() {
            if let Ok(raw) = fs::read_to_string(&path) {
                match serde_json::from_str::<DataWrapper>(&raw) {
                    Ok(wrapper) if !wrapper.maps.is_empty() => {
                        // Clamp a stale/corrupt active_index instead of panicking later.
                        let active_index = wrapper.active_index.min(wrapper.maps.len() - 1);
                        return (wrapper.maps, active_index);
                    }
                    Ok(_) => { /* empty map list: fall through to default */ }
                    Err(_) => {
                        // Back the unreadable file up instead of silently
                        // overwriting the user's data on the next auto-save.
                        let backup = Self::dir().join("data.json.bak");
                        fs::copy(&path, &backup).ok();
                    }
                }
            }
        }
        // Default: one empty map
        let mut mm = MindMap::new();
        mm.name = "Default".to_string();
        (vec![mm], 0)
    }

    pub fn save(maps: &[MindMap], active_index: usize) {
        fs::create_dir_all(Self::dir()).ok();
        let wrapper = DataWrapperRef { maps, active_index };
        if let Ok(json) = serde_json::to_string_pretty(&wrapper) {
            fs::write(Self::data_path(), json).ok();
        }
    }
}
