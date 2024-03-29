use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use crate::theme::ThemeValue;
use anyhow::{Context, Result};
use serde::Deserialize;

pub const CONFIG_FILE_PATH: &str = "lirstings.json";

#[derive(Deserialize, Clone, Hash, Debug)]
pub struct Config {
    pub theme: BTreeMap<String, ThemeValue>,
    pub query_search_dirs: Vec<String>,
    pub parser_search_dirs: Vec<PathBuf>,
    pub ansi_colors: Vec<String>,
    pub comment_map: BTreeMap<String, CommentStyle>,
}

#[derive(Deserialize, Clone, Hash, Debug)]
pub struct CommentStyle {
    pub line: String,
    pub block: (String, String),
}

impl Config {
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

    pub fn resolve_links(&mut self) -> Result<()> {
        let mut must_reresolve = false;
        let mut replacements = vec![];
        for (key, value) in self.theme.iter() {
            let link_key = match value {
                ThemeValue::Color(str) if str.starts_with('$') => &str[1..],
                ThemeValue::Object {
                    link: Some(str), ..
                } => str,
                _ => continue,
            };
            let resolved = value.linked_to(
                self.theme
                    .get(link_key)
                    .with_context(|| format!("link to unknown key `{link_key}`"))?,
            );
            if matches!(&resolved, ThemeValue::Color(str) if str.starts_with('$'))
                || matches!(&resolved, ThemeValue::Object { link: Some(_), .. })
            {
                must_reresolve = true;
            }
            replacements.push((key.clone(), resolved));
        }
        for (key, replacement) in replacements {
            *self
                .theme
                .get_mut(&key)
                .expect("key validity checked above") = replacement;
        }
        if must_reresolve {
            self.resolve_links()?;
        }
        Ok(())
    }
}
