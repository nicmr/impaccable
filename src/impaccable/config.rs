use std::{path::{PathBuf, Path}, collections::{HashMap, BTreeSet, BTreeMap, btree_map::Entry}};

use anyhow::{Context, anyhow, bail};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::io::Write;
use crate::impaccable;

use super::{GroupId, Error, PackageId, PackageGroup, PackageGroupMap};

use std::iter::Extend;

// TODO(high, docs): ensure, then document that config_path is always an absolute path
/// Manages the configuration, persisting it to the config file when required.
pub struct ConfigManager {
    config_path: PathBuf,
    config: Config,
}

impl ConfigManager {
    pub fn new(config_path: PathBuf, config: Config) -> Self {
        Self {
            config_path,
            config,
        }
    }

    pub fn config(&self) -> &Config { &self.config }

    /// Adds a new root group to the specified target configuration.
    /// Returns `false` if no change was performed, i.e. the group was already present.
    pub fn add_root_group(&mut self, target_id: &TargetId, group: GroupId) -> anyhow::Result<bool> {
        if let Some(target_config) = self.config.targets.get_mut(target_id) {
            let added = target_config.root_groups.insert(group);
            self.write_config_to_disk()?;
            Ok(added)
        } else {
            bail!(Error::TargetNotFound(target_id.clone()))
        }
    }

    pub fn absolute_package_dir(&self) -> anyhow::Result<PathBuf> {
        self.config_path
            .parent()
            .ok_or(anyhow!("Failed to get directory containing config"))
            .map(|dir| dir.join(&self.config.package_dir))
    }

    pub fn parse_package_configuration(&self) -> anyhow::Result<PackageConfiguration> {
        let absolute_package_dir = self.absolute_package_dir()?;
        PackageConfiguration::parse(&absolute_package_dir).context("failed to parse package configuration")
    }

    fn write_config_to_disk(&self) -> anyhow::Result<()> {
        let serialized_config = toml::to_string_pretty(&self.config)?;
        let mut file = std::fs::File::create(&self.config_path)?;
        write!(file, "{}", serialized_config)?;
        Ok(())

    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub package_dir: PathBuf,
    pub targets: BTreeMap<TargetId, TargetConfig>
}

impl Config {
    /// Creates an instance of `Self` with placeholder values to template the config file.
    /// 
    /// Reads some system-specific data for user-friendly initial values:
    /// * `/etc/hostname` as default target name
    /// 
    /// Will return `Err` if retrieval of system-specific data fails
    pub fn template() -> anyhow::Result<Self> {
        let mut targets = BTreeMap::new();

        let hostname = std::fs::read_to_string("/etc/hostname")?;
        targets.insert(
            hostname,
            TargetConfig { root_groups: [String::from("awesome_software")].into() }
        );
        Ok(Self {
                    package_dir : "./packages".into(),
                    targets,
                })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TargetConfig {
    pub root_groups: BTreeSet<GroupId>
}

/// Represents the parsed form of the entire package configuration of a system.
/// Files are indexed by their absolute paths.
#[derive(Debug, Default, Clone)]
pub struct PackageConfiguration {
    pub files : HashMap<PathBuf, PackageFile>
}

#[derive(Debug, Default, Clone)]
pub struct PackageFile {
    pub groups: PackageGroupMap
}

impl PackageFile {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new()
        }
    }
    pub fn from_groups(groups: PackageGroupMap) -> Self {
        Self { groups }
    }
}

impl PackageConfiguration{
    /// Parses a package directory to generate a corresponding `PackageConfiguration`
    fn parse(package_dir: &Path) -> impaccable::Result<Self> {
        let mut package_configuration = PackageConfiguration::default();

        for entry in WalkDir::new(package_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
            
            let path = entry.path();
            let file_string = std::fs::read_to_string(path)?;
            let groups: PackageGroupMap = toml::from_str(&file_string)?;

            if let Some(_duplicate_file) = package_configuration.files.insert(path.to_owned(), PackageFile { groups }) {
                return Err(Error::PackageFileAlreadyExists { package_file: path.to_path_buf() })
            }
        }
        Ok(package_configuration)
    }

    /// Creates a new group in the package file at the specified `file_path`.
    /// Returns Err if the file does not exist or the group alreaddy exists in said file
    pub fn create_group(&mut self, group_id: GroupId, file_path: &Path) -> impaccable::Result<()> {
        let Some(package_file) = self.files.get_mut(file_path) else {
            return Err(Error::PackageFileNotFound{ package_file: file_path.to_owned()})
        };

        if let Entry::Vacant(e) = package_file.groups.entry(group_id.clone()){
            e.insert(PackageGroup::new());
            Ok(())
        }
        else {
            Err(Error::GroupAlreadyExists { group: group_id })
        }
    }

    /// Returns an iterator over the packages contained by the specified groups.
    pub fn packages_of_groups<'a>(&'a self, groups: &'a BTreeSet<GroupId>) -> impl Iterator<Item = &PackageId> + 'a  {
        self.groups(groups)
            .flat_map(|(_, package_group)| &package_group.members)
    }

    /// Creates an iterator over package groups pre-filtered to only contain the specified groups.
    pub fn groups<'a>(&'a self, groups: &'a BTreeSet<GroupId>) -> impl Iterator<Item = (&String, &PackageGroup)> {
        self.files
            .iter()
            .flat_map(|(_file_name, contents)| &contents.groups)
            .filter(|(group_name, _)| groups.contains(*group_name))
    }

    /// Creates a new package configuration file at the specified path.
    /// If no contents are passed, the file will be created empty.
    // TODO: check that supplied path is inside package directory (or use special wrapper type (`PathInsidePackageDir`) that makes that guarantee?)
    pub fn create_file(&mut self, file_path_abs: &Path, opt_contents: Option<PackageGroupMap> ) -> impaccable::Result<()>{
        
        if !self.files.contains_key(file_path_abs) {

            let package_file = match opt_contents {
                Some(groups) => PackageFile::from_groups(groups),
                None => PackageFile::new(),
            };

            self.files.insert(file_path_abs.to_owned(), package_file);
            self.write_file_to_disk(file_path_abs)?;
            Ok(())
        } else {
            Err(Error::PackageFileAlreadyExists { package_file: file_path_abs.to_owned() })
        }
    }


    pub fn iter_groups(&self) -> impl Iterator<Item = (&GroupId, &PackageGroup)> {
        self.files.iter()
            .flat_map(|(_file_name, contents)| &contents.groups)
    }

    /// Adds packages to the specified group
    pub fn add_packages<I>(&mut self, packages: I, group_id: &GroupId) -> impaccable::Result<()>
    where
        I: IntoIterator<Item = String>
    {
        let mut package_group_ref : Option<&mut BTreeSet<PackageId>> = Option::None;
        let mut file_to_save : Option<PathBuf> = Option::None;

        'outer: for (filepath, package_file) in self.files.iter_mut() {
            for (group_name, g) in package_file.groups.iter_mut() {
                if group_name == group_id {
                    package_group_ref = Some(&mut g.members);
                    file_to_save = Some(filepath.clone());
                    break 'outer;
                }
            }
        }
        let (Some(package_group), Some(file)) = (package_group_ref, file_to_save) else {
            return Err(Error::GroupNotFound { group: group_id.clone() });
        };
        package_group.extend(packages);
        self.write_file_to_disk(&file)?;
        Ok(())
    }

    /// Removes a package from a group.
    pub fn remove_package(&mut self, package_id: &PackageId, group_id: &GroupId) -> impaccable::Result<()> {
        let mut file_to_save : Option<PathBuf> = Option::None;
        let mut group_found = false;

        'outer: for (filepath, package_file) in self.files.iter_mut() {
            for (group_name, g) in package_file.groups.iter_mut() {
                if group_name == group_id {
                    group_found = true;
                    if g.members.remove(package_id) {
                        file_to_save = Some(filepath.clone());
                    }
                    break 'outer;
                }
            }
        }
        let Some(file) = file_to_save else {
            if !group_found {
                return Err(Error::GroupNotFound { group: group_id.to_owned() });
            } else {
                return Err(Error::PackageNotFound { package: package_id.clone() });
            }
        };
        self.write_file_to_disk(&file)?;
        Ok(())
    }

    // Write a file with its currently configured groups to disk.
    // Will create the file if it exists or truncate otherwise.
    fn write_file_to_disk(&self, file_path: &Path) -> impaccable::Result<()> {
        let Some(group_file) = self.files.get(file_path) else {
            return Err(Error::PackageFileNotFound { package_file: file_path.to_owned() });
        };
        let serialized_groups = toml::to_string_pretty(&group_file.groups)?;
        let mut file = std::fs::File::create(file_path)?;
        write!(file, "{}", serialized_groups)?;
        Ok(())
    }
    
}
pub type TargetId = String;

// Manages the active Target of the system. Will write to the active target file on configuration change
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveTarget {
    target : TargetId,
}

impl ActiveTarget {
    pub fn new(target: TargetId) -> Self {
        Self {target}
    }

    pub fn parse(s: &str) -> Result<Self, Error> {
        let active_target: Self = toml::from_str(s)?;
        Ok(active_target)
    }

    pub fn target(&self) -> &TargetId {
        &self.target
    }

    // TODO(medium): nicer interface, not so intuitive having to pass the path here. Same approach as ConfigManager?
    /// Changes the active target and writes it to the specified file
    pub fn set_target(&mut self, target: TargetId, path: &Path, ) -> Result<(), Error> {
        self.target = target;
        let serialized_target = toml::to_string_pretty(self)?;
        let mut file = std::fs::File::create(path)?;
        write!(file, "{}", serialized_target)?;
        Ok(())
    }
}
