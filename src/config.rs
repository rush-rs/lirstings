use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::theme::ThemeValue;

#[derive(serde::Deserialize)]
pub struct Config {
    pub theme: HashMap<String, ThemeValue>,
    pub query_search_dirs: Vec<String>,
    pub parser_search_dirs: Vec<PathBuf>,
}

pub fn read(file_path: &str) -> Result<Option<Config>> {
    // either read or create a configuration file based on it's current existence
    let path = Path::new(file_path);
    match &path.exists() {
        true => {
            // the file exists, it can be read
            let config_file = File::open(file_path)?;
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
