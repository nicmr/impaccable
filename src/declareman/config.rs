use std::{path::{PathBuf, Path}, collections::{HashMap, BTreeSet}};

use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use std::io::Write;

use super::{GroupId, TargetConfig, PackageConfiguration, DeclaremanError, GroupMap, PackageId, PackageGroup};

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

        PackageConfiguration::parse(&absolute_package_dir)
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeclaremanConfig {
    pub root_group: GroupId,
    pub package_dir: PathBuf,
    pub targets: HashMap<TargetId, TargetConfig>
}

impl DeclaremanConfig {
    /// Creates an instance of `Self` with placeholder values to template the config file
    /// 
    pub fn template() -> Self {
        let mut targets = HashMap::new();
        // TODO: use /etc/hostname or gethostname to insert the hostname of the machine as the default value
        targets.insert(
            String::from("my_arch_machine"),
            TargetConfig { root_groups: [String::from("awesome_software")].into() }
        );
        Self {
            root_group : "myrootgroup".into(),
            package_dir : "./packages".into(),
            targets,
        }
    }
}



impl PackageConfiguration {
    fn parse(package_dir: &Path) -> anyhow::Result<Self> {
        let mut package_configuration = PackageConfiguration::default();
        for entry in WalkDir::new(&package_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) {
            
            let file_string = std::fs::read_to_string(entry.path())?;
            let groups_in_file: GroupMap = toml::from_str(&file_string)?;
            for (group_name, group_values) in groups_in_file.into_iter() {
                package_configuration.files.add_group(entry.path().to_path_buf(), group_name.clone());

                // TODO: ensure group name is not already defined
                package_configuration.groups.insert(group_name, group_values);
            }
        }
        Ok(package_configuration)
    }

    /// Returns a list of all installed packages
    pub fn installed_packages(&self) -> BTreeSet<PackageId> {
        self.groups.values()
            .cloned()
            .map(|package_group| package_group.members)
            .flatten()
            .collect()
    }

    /// Adds a package to the specified group
    pub fn add_packages(&mut self, packages: BTreeSet<String>, group_id: &GroupId) -> Result<(), DeclaremanError> {
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

    // TODO: consider passing optional groupid to scope to group
    /// Returns the ids of the groups it was removed from
    pub fn remove_package(&mut self, package: &GroupId) -> BTreeSet<GroupId> {

        let removed_from_groups : BTreeSet<GroupId> = self.groups.iter_mut()
            .filter_map(|(group_id, group)| {
                if group.members.remove(package) { Some(group_id.clone()) }
                else { None }
            }).collect();

        removed_from_groups
    }

    fn save_group_to_file(&self, group_id: &GroupId) -> Result< (), DeclaremanError> {
        // let (file, groups)
        match self.files.groups_in_same_file(group_id) {
            None => Err(DeclaremanError::GroupNotFound { group: group_id.to_string()}),
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

    pub fn parse(s: &str) -> Result<Self, DeclaremanError> {
        let active_target: Self = toml::from_str(s)?;
        Ok(active_target)
    }

    pub fn target(&self) -> &TargetId {
        &self.target
    }

    // TODO: should this check that the target exists? take target map as argument?
    // TODO: nicer interface, not so intuitive having to pass the path here
    /// Changes the active target and writes it to the specified file
    pub fn set_target(&mut self, target: TargetId, path: &Path, ) -> Result<(), DeclaremanError> {
        self.target = target;
        let serialized_target = toml::to_string_pretty(self)?;
        let mut file = std::fs::File::create(path)?;
        write!(file, "{}", serialized_target)?;
        Ok(())
    }
}