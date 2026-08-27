use std::{
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use xdg::BaseDirectories;

const DIR_NAME: &str = "brunch";

#[derive(Debug, Error)]
pub enum DirectoriesError {
    #[error("could not create config directory: {0}")]
    Config(#[source] io::Error),
    #[error("could not create data directory: {0}")]
    Data(#[source] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirectories {
    config: PathBuf,
    data: PathBuf,
}

impl AppDirectories {
    pub fn initialize() -> Result<Self, DirectoriesError> {
        let directories = BaseDirectories::with_prefix(DIR_NAME);
        let config = directories
            .create_config_directory("")
            .map_err(DirectoriesError::Config)?;
        let data = directories
            .create_data_directory("")
            .map_err(DirectoriesError::Data)?;

        Ok(Self { config, data })
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }
}
