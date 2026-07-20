//! Storage in config.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

/// Name of the local sqlite cache file under `root`.
const CACHE: &str = "Problems";

/// Locate code files
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Storage {
    code: String,
    root: String,
    scripts: Option<String>,
    /// Directory for `*.tests.dat` files. Relative to `root`, or absolute.
    /// When omitted, falls back to the same directory as `code` (upstream behavior).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tests: Option<String>,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            code: "code".into(),
            scripts: Some("scripts".into()),
            root: "~/.leetcode".into(),
            tests: None,
        }
    }
}

impl Storage {
    /// convert root path
    pub fn root(&self) -> Result<String> {
        let home = dirs::home_dir()
            .ok_or(Error::NoneError)?
            .to_string_lossy()
            .to_string();
        let path = self.root.replace('~', &home);
        Ok(path)
    }

    /// Resolve a storage sub-path: expand `~`, join under root when relative,
    /// create the directory if missing.
    fn resolve_dir(&self, sub: &str) -> Result<String> {
        let home = dirs::home_dir()
            .ok_or(Error::NoneError)?
            .to_string_lossy()
            .to_string();
        let sub = sub.replace('~', &home);
        let p = {
            let candidate = PathBuf::from(&sub);
            if candidate.is_absolute() {
                candidate
            } else {
                PathBuf::from(self.root()?).join(sub)
            }
        };
        if !p.exists() {
            fs::DirBuilder::new().recursive(true).create(&p)?;
        }
        Ok(p.to_string_lossy().to_string())
    }

    /// get cache path
    pub fn cache(&self) -> Result<String> {
        let root = PathBuf::from(self.root()?);
        if !root.exists() {
            info!("Generate cache dir at {:?}.", &root);
            fs::DirBuilder::new().recursive(true).create(&root)?;
        }

        Ok(root.join(CACHE).to_string_lossy().to_string())
    }

    /// get code path
    pub fn code(&self) -> Result<String> {
        self.resolve_dir(&self.code)
    }

    /// Directory for generated `*.tests.dat` sample inputs.
    /// Defaults to the code directory when `storage.tests` is unset.
    pub fn tests(&self) -> Result<String> {
        match &self.tests {
            Some(t) => self.resolve_dir(t),
            None => self.code(),
        }
    }

    /// get scripts path
    pub fn scripts(mut self) -> Result<String> {
        if self.scripts.is_none() {
            self.scripts = Some("scripts".into());
        }
        let sub = self.scripts.clone().ok_or(Error::NoneError)?;
        self.resolve_dir(&sub)
    }
}