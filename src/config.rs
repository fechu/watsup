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
            data_store: home.join("Library/Application Support/watson"),
        }
    }
}
