use std::{collections::BTreeMap, fmt::Display};

use anyhow::bail;

use crate::impaccable::{PackageGroupMap, PackageGroup};

const ENDEAVOUR_OS : &str = "EndeavourOS";

pub fn get_system_configuration() -> anyhow::Result<SystemConfiguration> {
    let os_release = os_release::OsRelease::new()?;
    let distro = os_release.name;

    match distro.as_str() {
        ENDEAVOUR_OS => {
            // encapsule in own error type so this can be handled predictably
            let desktop = std::env::var("XDG_CURRENT_DESKTOP")?;
            Ok(SystemConfiguration {
                distro,
                desktop
            })
        },
        _ => bail!(format!("Distro not supported for package templating: {}", &distro)),
    }
}

pub fn generate_configuration(system_config: &SystemConfiguration) -> anyhow::Result<PackageGroupMap> {
    match system_config.distro.as_str() {
        ENDEAVOUR_OS => {
            let eos_package_list_base_url = "https://raw.githubusercontent.com/endeavouros-team/EndeavourOS-packages-lists/master/";
            let url_path_base = "eos-base-group";

            let mut group_map : PackageGroupMap = BTreeMap::new();

            for url_path in &[url_path_base, &system_config.desktop] {

                let package_url = format!("{}{}", eos_package_list_base_url, url_path);

                // TODO(medium, optimization): explore possibilities to run as async instead
                let response = reqwest::blocking::get(package_url)?.text()?;
                println!("{}", response);

                let package_group = PackageGroup {
                    members: response.lines().map(|x| x.to_owned()).collect()
                };
                group_map.insert(format!("{}-{}", system_config.distro, url_path), package_group);
            }
            Ok(group_map)
        }
        _ => {
            bail!(format!("Distro not supported for package templating: {}", &system_config.distro))
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemConfiguration {
    pub distro: String,
    pub desktop: String,
}

impl Display for SystemConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on {}", self.distro, self.desktop)
    }
}