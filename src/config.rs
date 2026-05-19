use std::{env, io, path::PathBuf};

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

    pub fn ensure_data_store_folder_exists(&self) -> Result<(), io::Error> {
        if !self.data_store.is_dir() {
            std::fs::create_dir(&self.data_store)?;
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = PathBuf::from(env::var("HOME").unwrap());
        let data_store_path = match std::env::consts::OS {
            "macos" => home.join("Library/Application Support/watson"),
            "linux" => home.join(".config/watson"),
            _ => "/tmp/".into(),
        };
        Self {
            data_store: data_store_path,
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
