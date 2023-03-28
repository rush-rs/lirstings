use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::Result;
use lirstings::theme::Theme;
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_PATH: &str = "lirstings.json";

#[derive(Serialize, Deserialize, Hash, Clone)]
pub struct Config {
    pub query_search_dirs: Vec<String>,
    pub parser_search_dirs: Vec<PathBuf>,
    pub theme: Theme,
}

impl Config {
    pub fn read() -> Result<Option<Self>> {
        // either read or create a configuration file based on it's current existence
        let path = Path::new(CONFIG_FILE_PATH);
        if path.exists() {
            // the file exists, it can be read
            let config_file = File::open(CONFIG_FILE_PATH)?;
            let config_file_reader = BufReader::new(config_file);
            let config = serde_json::from_reader(config_file_reader)?;
            Ok(Some(config))
        } else {
            // The file does not exist, therefore create a new one
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, include_bytes!("default_config.json"))?;
            Ok(None)
        }
    }
}
