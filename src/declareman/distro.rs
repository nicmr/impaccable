use std::collections::HashMap;

use anyhow::bail;

use crate::declareman::PackageGroup;

use super::GroupMap;

const ENDEAVOUR_OS : &'static str = "EndeavourOS";

/// Custom distro support for templating your packages

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
        // TODO: print detected distro
        _ => bail!("Distro not supported for package templating"),
    }
}

pub fn generate_configuration(system_config: &SystemConfiguration) -> anyhow::Result<GroupMap> {
    match system_config.distro.as_str() {
        ENDEAVOUR_OS => {
            let package_url =
              format!("https://raw.githubusercontent.com/endeavouros-team/EndeavourOS-packages-lists/master/{}", system_config.desktop);
            let response = reqwest::blocking::get(package_url)?.text()?;
            println!("{}", response);

            let mut group_map : GroupMap = HashMap::new();
            let package_group = PackageGroup {
                members: response.lines().map(|x| x.to_owned()).collect()
            };
            group_map.insert(format!("{}-{}", system_config.distro, system_config.desktop), package_group);
            Ok(group_map)
        }
        _ => {
            bail!("Distro not supported for package templating")
        }
    }

}

#[derive(Debug, Clone)]
pub struct SystemConfiguration {
    pub distro: String,
    pub desktop: String,
}