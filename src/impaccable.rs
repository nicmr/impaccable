use std::collections::{BTreeSet, BTreeMap};

use serde::{Deserialize, Serialize};

/// Configuration files
pub mod config;
/// Interaction with the pacman CLI
pub mod pacman;
/// Custom distro support for templating the package configurations
pub mod distro;
pub mod error;

pub use error::Error;
type Result<T> = std::result::Result<T, Error>;

pub type PackageId = String;
pub type GroupId = String;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageGroup {
    pub members: BTreeSet<PackageId>,
    pub accept_indirect: bool,
}

impl PackageGroup {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub fn from_members(members: BTreeSet<PackageId>) -> Self {
        Self {
            members,
            ..Default::default()
        }
    }
}

/// Default data structure to store Package groups 
pub type PackageGroupMap = BTreeMap<GroupId, PackageGroup>;
