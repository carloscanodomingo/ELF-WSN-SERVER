use crate::comm::influx_error::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{collections::HashMap, fmt, fs, mem};

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub url: String,
    pub port: String,
    pub org_id: String,
    pub bucket: String,
    pub token: String,
    pub devices: HashMap<String, String>,
}

impl ConfigFile {
    pub fn load_config(path: String) -> Result<ConfigFile, InfluxErrorCode> {
        let fs_read = fs::read_to_string(path);
        if let Err(_error) = fs_read {
            return Err(InfluxErrorCode::ConfigFileNotFound);
        }

        let config = fs_read.unwrap().to_string();

        let config_file: ConfigFile =
            serde_json::from_str(&config).map_err(|_| InfluxErrorCode::NotValidConfigFile)?;
        Ok(config_file)
    }
}
