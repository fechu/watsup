use std::{env, path::PathBuf};

pub struct Config {
    data_store: PathBuf,
}

impl Config {
    pub fn get_state_path(&self) -> PathBuf {
        self.data_store.join("state")
    }

    pub fn get_frames_path(&self) -> PathBuf {
        self.data_store.join("frames")
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = PathBuf::from(env::var("HOME").unwrap());
        Self {
            data_store: match std::env::consts::OS {
                "macos" => home.join("Library/Application Support/watson"),
                "linux" => home.join(".config/watson"),
                _ => "/tmp/".into(),
            },
        }
    }
}

#[cfg(test)]
impl Config {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            data_store: storage_path,
        }
    }
}
