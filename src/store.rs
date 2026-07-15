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

pub struct Store;

impl Store {
    fn dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap();
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
                if let Ok(wrapper) = serde_json::from_str::<DataWrapper>(&raw) {
                    return (wrapper.maps, wrapper.active_index);
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
        let wrapper = DataWrapper {
            maps: maps.to_vec(),
            active_index,
        };
        if let Ok(json) = serde_json::to_string_pretty(&wrapper) {
            fs::write(Self::data_path(), json).ok();
        }
    }
}
