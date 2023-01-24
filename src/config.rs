use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use crate::theme::ThemeValue;
use anyhow::Result;
use serde::Deserialize;

pub const CONFIG_FILE_PATH: &str = "lirstings.json";

#[derive(Deserialize, Clone, Hash)]
pub struct Config {
    pub theme: BTreeMap<String, ThemeValue>,
    pub query_search_dirs: Vec<String>,
    pub parser_search_dirs: Vec<PathBuf>,
    pub ansi_colors: Vec<String>,
}

pub fn read() -> Result<Option<Config>> {
    // either read or create a configuration file based on it's current existence
    let path = Path::new(CONFIG_FILE_PATH);
    match &path.exists() {
        true => {
            // the file exists, it can be read
            let config_file = File::open(CONFIG_FILE_PATH)?;
            let config_file_reader = BufReader::new(config_file);
            let config: Config = serde_json::from_reader(config_file_reader)?;
            Ok(Some(config))
        }
        false => {
            // The file does not exist, therefore create a new one
            fs::create_dir_all(path.parent().unwrap())?;
            let mut file = File::create(path)?;
            file.write_all(include_bytes!("default_config.json"))?;
            Ok(None)
        }
    }
}
