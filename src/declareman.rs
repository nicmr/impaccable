use std::{collections::{HashMap, BTreeSet, BTreeMap}, process::{Command, Stdio}};
use anyhow::Context;

use serde::{Deserialize, Serialize};

pub mod pacman;
pub mod distro;
pub mod config;

pub mod error;

pub use error::Error;
type Result<T> = std::result::Result<T, Error>;

// TODO: add custom result type. Maybe with shorter name?
// pub type DeclaremanResult<T> = Result<T, DeclaremanError>;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub root_groups: Vec<GroupId>
}




pub type PackageId = String;

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

pub type GroupId = String;

// TODO(hight): decide whether to use this alias or not
type GroupMap = BTreeMap<GroupId, PackageGroup>;

#[derive(Debug, Serialize, Deserialize)]
struct Target {
    pub name: String,
    pub root_group: GroupId,
}



// TODO: move to pacman module
pub fn install_group_packages(packages: &BTreeMap<GroupId, PackageGroup>, install_group: &str) -> anyhow::Result<()> {
    if let Some(root_group) = packages.get(install_group) {
        let _pacman_command = Command::new("pacman")
            .arg("-S")
            .args(&root_group.members)
            .stdin(Stdio::inherit())
            .status()
            .context("Failed to run pacman")?;
        Ok(())
    } else {
        Err(anyhow::Error::from(Error::RootPackageNotFound(String::from(install_group))))
    }
}

pub fn group_intersection(packages_by_group: HashMap<String, PackageGroup>, with_set: &BTreeSet<String>) -> HashMap<String, PackageGroup> {
    let mut intersections_by_group = HashMap::with_capacity(packages_by_group.len());

    // todo: more efficient map, maybe iter_mut implementation instead
    for (group, packages) in packages_by_group {
        intersections_by_group.insert(group, 
            PackageGroup {
                members: packages.members.intersection(with_set).cloned().collect()
            }
        );
    }

    intersections_by_group
}
