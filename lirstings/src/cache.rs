use std::{hash::{Hash, Hasher}, any::{Any, TypeId}};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;

use crate::{theme::Theme, Mode};

pub const CACHE_SKIP_MESSAGE: &str = "skipping generation of cached input";
pub const CACHE_WRITE_MESSAGE: &str = "written to cache";

#[derive(Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct CacheKey(u64);

impl CacheKey {
    pub fn new<T: Any>(mode: &Mode, code: &str, theme: &Theme, additional: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        (mode, code, theme, TypeId::of::<T>(), additional).hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub trait Cache: Default {
    fn instantiate(&mut self) -> Result<()>;

    fn set_entry(&mut self, key: CacheKey, value: String) -> Result<()>;

    fn get_entry(&self, key: CacheKey) -> Option<&str>;
}

#[derive(Default)]
pub struct DummyCache;

impl Cache for DummyCache {
    fn instantiate(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_entry(&mut self, _key: CacheKey, _value: String) -> Result<()> {
        Ok(())
    }

    fn get_entry(&self, _key: CacheKey) -> Option<&str> {
        None
    }
}
