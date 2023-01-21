use std::{
    collections::HashMap,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{BufReader, Write},
    path::Path,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;

pub const CACHE_FILE_PATH: &str = "lirstings.cache.json";
pub const CACHE_SKIP_MESSAGE: &str = "lirstings: skipping generation of cached input";
pub const CACHE_WRITE_MESSAGE: &str = "lirstings: written to cache";

#[derive(Serialize, Deserialize, Default)]
pub struct Cache(HashMap<u64, String>);

impl Cache {
    pub fn set_entry(&mut self, hash: u64, output: String) -> Result<()> {
        self.0.insert(hash, output);

        let repr = serde_json::to_vec(self).with_context(|| "could not marshal cache struct")?;
        fs::write(CACHE_FILE_PATH, repr).with_context(|| "could not write to cache file")?;

        Ok(())
    }

    pub fn get_cached(&self, hash: u64) -> Option<&str> {
        self.0.get(&hash).map(|string| string.as_str())
    }
}

pub fn hash<T>(obj: T) -> u64
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    obj.hash(&mut hasher);
    hasher.finish()
}

pub fn read() -> Result<Cache> {
    // either read or create a configuration file based on it's current existence
    let path = Path::new(CACHE_FILE_PATH);
    match &path.exists() {
        true => {
            // the file exists, it can be read
            let file = File::open(CACHE_FILE_PATH)?;
            let file_reader = BufReader::new(file);
            let cache: Cache = serde_json::from_reader(file_reader)?;
            Ok(cache)
        }
        false => {
            // The file does not exist, therefore create a new one
            fs::create_dir_all(path.parent().unwrap())?;
            let mut file = File::create(path)?;
            let repr = serde_json::to_vec(&Cache::default())
                .with_context(|| "could not marshal default cache")?;
            file.write_all(&repr)?;
            Ok(Cache::default())
        }
    }
}
