use std::{io, path::PathBuf};

use thiserror::Error;

use super::{PackageId, GroupId, config::TargetId};

// /// Errors encountered when parsing the configuration
// // TODO: consider moving to config submodule
// pub enum ConfigError {
    
//     ConfigFileNotFound
// }

#[derive(Error, Debug)]
pub enum Error {
    // not found errors
    #[error("Active target `{0}` not found")]
    TargetNotFound(TargetId),

    #[error("Package file `{package_file}` not found")]
    PackageFileNotFound {
        package_file: PathBuf,
    },
    #[error("Group `{group}` not found")]
    GroupNotFound {
        group: GroupId,
    },

    #[error("Package `{package}` not found")]
    PackageNotFound {
        package: PackageId
    },


    // already exists errors
    #[error("Package file `{package_file}` already exists")]
    PackageFileAlreadyExists {
        package_file: PathBuf,
    },

    #[error("Group `{group}` already exists")]
    GroupAlreadyExists {
        group: GroupId,
    },

    #[error("Failed to open config file at `{path}`")]
    ConfigFileNotFound {
        path: PathBuf,
        source: io::Error,
    },

    // conversion errors
    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("Failed to serialize to toml")]
    SerializeFailure {
        #[from]
        source: toml::ser::Error,
    },
    #[error("Failed to deserialize from toml")]
    DeserializeFailure {
        #[from]
        source: toml::de::Error,
    },

    // other errors
    #[error("Config file has no parent directory")]
    ConfigFileHasNoParentDir {
        path: PathBuf
    },
}
