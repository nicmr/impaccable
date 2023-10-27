use std::collections::{BTreeSet, BTreeMap};

use serde::{Deserialize, Serialize};

pub mod pacman;
pub mod distro;
pub mod config;

pub mod error;

pub use error::Error;
type Result<T> = std::result::Result<T, Error>;

pub type PackageId = String;
pub type GroupId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    pub members: BTreeSet<PackageId>
}

impl PackageGroup {
    pub fn new() -> Self {
        Self {
            members: BTreeSet::new()
        }
    }
    pub fn from_members(members: BTreeSet<PackageId>) -> Self {
        Self { members }
    } 
}

// TODO(medium): decide whether to use this alias or not
type GroupMap = BTreeMap<GroupId, PackageGroup>;
