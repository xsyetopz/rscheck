use rscheck::config::Config as CoreConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Human,
    Json,
    Sarif,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "OutputConfig::default_format")]
    pub format: OutputFormat,
    #[serde(default)]
    pub output: Option<PathBuf>,
    #[serde(default = "OutputConfig::default_with_clippy")]
    pub with_clippy: bool,
}

impl OutputConfig {
    fn default_format() -> OutputFormat {
        OutputFormat::Human
    }

    fn default_with_clippy() -> bool {
        true
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Human,
            output: None,
            with_clippy: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(flatten)]
    pub core: CoreConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {path}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config file: {path}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to write config file: {path}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to serialize default config")]
    Serialize(#[source] toml::ser::Error),
}

impl FileConfig {
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str::<Self>(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

pub fn write_default_config(path: &Path) -> Result<(), ConfigError> {
    let config = FileConfig::default();
    let toml = toml::to_string_pretty(&config).map_err(ConfigError::Serialize)?;
    std::fs::write(path, toml).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

pub fn workspace_root() -> Result<PathBuf, cargo_metadata::Error> {
    let metadata = cargo_metadata::MetadataCommand::new().no_deps().exec()?;
    Ok(PathBuf::from(metadata.workspace_root.as_std_path()))
}

pub fn default_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".rscheck.toml")
}
