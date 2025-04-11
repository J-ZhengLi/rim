mod build_config;
mod tool_info;
mod tool_map;
mod toolkit_manifest;

// re-exports
pub use build_config::*;
pub use tool_info::*;
pub use tool_map::*;
pub use toolkit_manifest::*;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;
use toml::{de, ser};

use crate::utils;

pub trait TomlParser {
    const FILENAME: &'static str;

    /// Deserialize a certain type from [`str`] value.
    fn from_str(from: &str) -> Result<Self>
    where
        Self: Sized + DeserializeOwned,
    {
        Ok(de::from_str(from)?)
    }

    /// Serialize data of a type into [`String`].
    fn to_toml(&self) -> Result<String>
    where
        Self: Sized + Serialize,
    {
        Ok(ser::to_string(self)?)
    }

    /// Load TOML data directly from a certain file path.
    fn load<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        Self: Sized + DeserializeOwned,
    {
        let raw = utils::read_to_string("toml", path)?;
        Self::from_str(&raw)
    }

    /// Load data from certain file under the given `parent` directory.
    fn load_from_dir<P: AsRef<Path>>(parent: P) -> Result<Self>
    where
        Self: Sized + DeserializeOwned + Default,
    {
        let path = parent.as_ref().join(Self::FILENAME);
        Self::load(path)
    }

    /// Serialize the data and write to a file under `parent` directory.
    ///
    /// Note: Nothing will be written if the content of `self` is empty.
    fn write_to_dir<P: AsRef<Path>>(&self, parent: P) -> Result<()>
    where
        Self: Sized + Serialize,
    {
        let content = self.to_toml()?;
        if content.trim().is_empty() {
            return Ok(());
        }
        let path = parent.as_ref().join(Self::FILENAME);
        utils::write_file(path, &content, false)?;
        Ok(())
    }
}
