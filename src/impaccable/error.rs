use std::{io, path::PathBuf};

use thiserror::Error;

use super::{PackageId, GroupId, config::TargetId};

// TODO(low): split error into separate errors for package, group, file(?)
#[derive(Error, Debug)]
pub enum Error {
    // not found errors
    #[error("Active target `{0}` not found")]
    ActiveTargetNotFound(TargetId),

    #[error("Root group `{0}` not found")]
    RootGroupNotFound(GroupId),

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


    // conversion errors

    // TODO: create error implementation per use case instead?
    // https://kazlauskas.me/entries/errors
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
}
