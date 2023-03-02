use std::{collections::{HashMap, BTreeSet}, path::{PathBuf, Path}, process::{Command, Stdio}};
use std::io;
use anyhow::Context;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

// use std::io::prelude::*;
use std::io::Write;
use std::iter::Extend;


#[derive(Debug, Serialize, Deserialize)]
pub struct DeclaremanConfig {
    pub root_group: GroupId,
    pub package_dir: PathBuf,
}  


#[derive(Error, Debug)]
pub enum DeclareManError {
    #[error("Root package `{0}` not found")]
    RootPackageNotFound(String),
    // NotEnoughArguments("")
    #[error("Package {package} already in group {group}")]
    PackageAlreadyInGroup{
        package: String,
        group: String,
    },
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    // this should probably be an ordered data structure so the packages are always ordered
    // so rework to btreeset (?)
    pub members: BTreeSet<GroupId>
} 

impl PackageGroup {
    pub fn new(members: BTreeSet<GroupId>) -> Self {
        Self { members }
    } 
}

pub type GroupId = String;
type GroupMap = HashMap<GroupId, PackageGroup>;


#[derive(Debug, Default)]
pub struct PackageConfiguration {
    files: GroupFiles,
    pub groups: GroupMap,
}

impl PackageConfiguration {
    pub fn try_parse (config: &DeclaremanConfig, config_path: &Path) -> anyhow::Result<Self> {
        // TODO: check if path is relative first (might already be absolute)
        // TODO: remove unwrap
        let absolute_package_dir = config_path.parent().unwrap().join(&config.package_dir);
        println!("{:?}", absolute_package_dir);

        let mut package_configuration = PackageConfiguration::default();

        for entry in WalkDir::new(&absolute_package_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
            
            let file_string = std::fs::read_to_string(entry.path())?;
            let group_for_file: GroupMap = toml::from_str(&file_string)?;
            for (group_name, group_values) in group_for_file.into_iter() {
                package_configuration.files.add_group(entry.path().to_path_buf(), group_name.clone());

                // TODO: ensure group name is not already defined
                package_configuration.groups.insert(group_name, group_values);
            }
        }
        Ok(package_configuration)
    }

    pub fn add_packages(&mut self, packages: BTreeSet<String>, group_id: &GroupId) -> Result<(), DeclareManError> {
        match self.groups.get_mut(group_id) {
            Some(group) => {
                // group.members.push(package.to_string());
                group.members.extend(packages)
            }
            None => {
                self.groups.insert(
                    group_id.to_string(),
                    PackageGroup::new(packages)
                );
            },
        }
        self.save_group_to_file(group_id)?;
        Ok(())
    }

    fn save_group_to_file(&self, group_id: &GroupId) -> Result< (), DeclareManError> {
        // let (file, groups)
        match self.files.groups_in_same_file(group_id) {
            None => Err(DeclareManError::GroupNotFound { group: group_id.to_string()}),
            Some((file_path, group_ids)) => {

                let groups_to_write : GroupMap = group_ids.iter()
                    .filter_map(|group_id| self.groups.get_key_value(group_id))
                    .map(|(gid, g)| (gid.to_owned(), g.clone()))
                    .collect();

                let serialized_groups = toml::to_string_pretty(&groups_to_write)?;
                
                let mut file = std::fs::File::create(file_path)?;
                write!(file, "{}", serialized_groups)?;
                Ok(())
            }
        }
    }
}
    
    
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

// TODO:
// new data structure for groupfiles
// stores name of file and names of contained groups
// has to store hashmap in both directions
// groups -> filename to be able to look up the file a group is contained in
// filename -> groups to know which to serialize when rewriting the file
// when adding to group -> look up filename -> look up other groups in file -> serialize all


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
        Err(anyhow::Error::from(DeclareManError::RootPackageNotFound(String::from(install_group))))
    }
}