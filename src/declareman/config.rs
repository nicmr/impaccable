use std::{path::{PathBuf, Path}, collections::{HashMap, BTreeSet, BTreeMap, btree_map::Entry}};

use anyhow::Context;
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::io::Write;

use crate::declareman;

use super::{GroupId, TargetConfig, Error, PackageId, PackageGroup};

use std::iter::Extend;

pub struct DeclaremanConfigManager {
    config_path: PathBuf,
    config: DeclaremanConfig,
}

impl DeclaremanConfigManager {
    pub fn new(config_path: PathBuf, config: DeclaremanConfig) -> Self {
        Self {
            config_path,
            config,
        }
    }

    pub fn config(&self) -> &DeclaremanConfig { &self.config }
    pub fn config_path(&self) -> &Path { &self.config_path }

    // TODO: remove unwrap
    // TODO: think about - wouldn't it make things considerably easier if package dir config option
    //       was absolute without relative support for now
    pub fn absolute_package_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(self.config_path.parent().unwrap()
                .join(&self.config.package_dir))
    }

    pub fn package_configuration(&self) -> anyhow::Result<PackageConfiguration> {
        // TODO: check if path really is relative first (might already be absolute)

        let absolute_package_dir = self.absolute_package_dir()?;
        // println!("{:?}", absolute_package_dir); // make debug log

        PackageConfiguration::parse(&absolute_package_dir).context("failed to parse package configuration")
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeclaremanConfig {
    pub root_groups: BTreeSet<GroupId>,
    pub package_dir: PathBuf,
    pub targets: BTreeMap<TargetId, TargetConfig>
}

impl DeclaremanConfig {
    /// Creates an instance of `Self` with placeholder values to template the config file
    /// 
    pub fn template() -> Self {
        let mut targets = BTreeMap::new();
        // TODO: use /etc/hostname or gethostname to insert the hostname of the machine as the default value
        targets.insert(
            String::from("my_arch_machine"),
            TargetConfig { root_groups: [String::from("awesome_software")].into() }
        );
        Self {
            root_groups : BTreeSet::from(["myrootgroup".to_string()]),
            package_dir : "./packages".into(),
            targets,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PackageConfiguration {
    pub files : HashMap<PathBuf, PackageFile>
}

#[derive(Debug, Default, Clone)]
pub struct PackageFile {
    pub groups: BTreeMap<GroupId, PackageGroup>
}

impl PackageFile {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new()
        }
    }
}

impl PackageConfiguration{
    /// Parses a package directory to generate a corresponding `PackageConfiguration`
    fn parse(package_dir: &Path) -> declareman::Result<Self> {
        let mut package_configuration = PackageConfiguration::default();

        for entry in WalkDir::new(package_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
            
            let path = entry.path();
            let file_string = std::fs::read_to_string(path)?;
            let groups: BTreeMap<String, PackageGroup> = toml::from_str(&file_string)?;

            if let Some(_duplicate_file) = package_configuration.files.insert(path.to_owned(), PackageFile { groups }) {
                return Err(Error::PackageFileAlreadyExists { package_file: path.to_path_buf() })
            }
        }
        Ok(package_configuration)
    }

    /// Creates a new group in the package file at the specified `file_path`.
    /// Returns Err if the file does not exist or the group alreaddy exists in said file
    pub fn create_group(&mut self, group_id: &GroupId, file_path: &Path) -> declareman::Result<()> {
        let Some(package_file) = self.files.get_mut(file_path) else {
            return Err(Error::PackageFileNotFound{ package_file: file_path.to_owned()})
        };

        if let Entry::Vacant(e) = package_file.groups.entry(group_id.clone()){
            e.insert(PackageGroup::new());
            Ok(())
        }
        else {
            Err(Error::GroupAlreadyExists { group:group_id.to_owned() })
        }
    }

    // TODO(low, ergonomics): consider taking iterator
    // TODO(low, errors): try to return declareman::Result instead
    pub fn packages_of_groups<'a>(&'a self, groups: &'a BTreeSet<GroupId>) -> impl Iterator<Item = &PackageId> + 'a  {
        self.files
            .iter()
            .flat_map(|(_file_name, contents)| &contents.groups)
            .filter(|(group_name, _)| groups.contains(*group_name))
            .flat_map(|(_, package_group)| &package_group.members)
    }

    // TODO(medium): implement group removal
    /// Removes a group
    // pub fn remove_group(&mut self, group_id: &GroupId) -> declareman::Result<bool> {
    // }

    /// Creates a new package configuration file at the specified path
    pub fn create_file(&mut self, file_path: &Path) -> declareman::Result<()>{
        if !self.files.contains_key(file_path) {
            self.files.insert(file_path.to_owned(), PackageFile::new());
            Ok(())
        } else {
            Err(Error::PackageFileAlreadyExists { package_file: file_path.to_owned() })
        }
    }


    pub fn iter_groups(&self) -> impl Iterator<Item = (&GroupId, &PackageGroup)> {
        self.files.iter()
            .flat_map(|(_file_name, contents)| &contents.groups)
    }

    // /// Allows for flattened iteration over all packages.
    // /// If you need information about groups or files, access via the field.
    // pub fn iter_packages(&self) -> impl Iterator<Item = &PackageId>{
    //     self.files
    //         .iter()
    //         .flat_map(|(_file_name, contents)| &contents.groups)
    //         .flat_map(|(_group_name, group) | &group.members)
    // }

    /// Adds packages to the specified group
    pub fn add_packages<I>(&mut self, packages: I, group_id: &GroupId) -> declareman::Result<()>
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

    // TODO(medium, ergonomics): decide whether to change API to accept multiple packages
    /// Removes a package from a group.
    pub fn remove_package(&mut self, package_id: &PackageId, group_id: &GroupId) -> declareman::Result<()> {
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

    // Write a file with its currently configured groups to disk
    fn write_file_to_disk(&self, file_path: &Path) -> declareman::Result<()> {
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

    // TODO: should this check that the target exists? take target map as argument?
    // TODO: nicer interface, not so intuitive having to pass the path here
    /// Changes the active target and writes it to the specified file
    pub fn set_target(&mut self, target: TargetId, path: &Path, ) -> Result<(), Error> {
        self.target = target;
        let serialized_target = toml::to_string_pretty(self)?;
        let mut file = std::fs::File::create(path)?;
        write!(file, "{}", serialized_target)?;
        Ok(())
    }
}
