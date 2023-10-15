use std::{collections::{HashMap, BTreeSet}, path::{PathBuf, Path}, process::{Command, Stdio}};
use std::io;
use anyhow::Context;
use thiserror::Error;
use serde::{Deserialize, Serialize};

pub mod pacman;
pub mod distro;
pub mod config;

// TODO: add custom result type. Maybe with shorter name?
// pub type DeclaremanResult<T> = Result<T, DeclaremanError>;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub root_groups: Vec<GroupId>
}



#[derive(Error, Debug)]
pub enum DeclaremanError {
    #[error("Root package `{0}` not found")]
    RootPackageNotFound(String),
    // NotEnoughArguments("")
    #[error("Group `{group}` not found")]
    GroupNotFound {
        group: String,
    },
    // TODO: create error implementation per use case instead
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

pub type PackageId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    pub members: BTreeSet<PackageId>
} 

impl PackageGroup {
    pub fn new(members: BTreeSet<PackageId>) -> Self {
        Self { members }
    } 
}

pub type GroupId = String;
type GroupMap = HashMap<GroupId, PackageGroup>;

/// Stores the files package groups are stored in. 
/// Bidirectional, whereas 1 file maps to N groups
#[derive(Debug, Default)]
struct GroupFiles {
    group_to_file: HashMap<GroupId, PathBuf>,
    file_to_groups: HashMap<PathBuf, Vec<GroupId>>,
}

impl GroupFiles {
    fn add_group(&mut self, file_path: PathBuf, group_name: GroupId) {
        self.group_to_file.insert(group_name.clone(), file_path.clone());

        match self.file_to_groups.get_mut(&file_path) {
            Some(groups) => groups.push(group_name),
            None => {self.file_to_groups.insert(file_path, vec![group_name]);},
        }
    }

    fn file(&self, group: &GroupId) -> Option<&PathBuf> {
        self.group_to_file.get(group)
    }

    fn groups(&self, file_path: &Path) -> Option<&Vec<GroupId>> {
        self.file_to_groups.get(file_path)
    }

    /// Returns the containing file and all groups in the same file
    fn groups_in_same_file(&self, group: &GroupId) -> Option<(&PathBuf, &Vec<GroupId>)> {
        let file = self.file(group)?;
        self.groups(file).map(|groups| (file, groups))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Target {
    pub name: String,
    pub root_group: GroupId,
}

pub fn install_packages(packages: &HashMap<String, PackageGroup>, install_group: &str) -> anyhow::Result<()> {
    if let Some(root_group) = packages.get(install_group) {
        let _pacman_command = Command::new("pacman")
            .arg("-S")
            .args(&root_group.members)
            .stdin(Stdio::inherit())
            .status()
            .context("Failed to run pacman")?;
        Ok(())
    } else {
        Err(anyhow::Error::from(DeclaremanError::RootPackageNotFound(String::from(install_group))))
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
