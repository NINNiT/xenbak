#![allow(dead_code)]
use crate::config::LocalStorageConfig;

use super::StorageHandler;

pub struct LocalStorage {
    pub path: String,
}

impl LocalStorage {
    pub fn from_config(config: LocalStorageConfig) -> Self {
        LocalStorage { path: config.path }
    }

    pub fn new() -> Self {
        LocalStorage {
            path: String::default(),
        }
    }
}
