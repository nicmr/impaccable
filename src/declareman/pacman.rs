use std::{collections::BTreeSet, process::{Command, Stdio, ExitStatus}, ffi::OsStr, sync::OnceLock};

use anyhow::{Context, bail};
use pomsky_macro::pomsky;
use regex::Regex;

const RE_PACKAGE_REQUIRED_BY: &str = pomsky!(
    let package_name_char = ['a'-'z' '0'-'9' '@' '.' '_' '+' '-'];
    "Required By"[s]+": ":(((package_name_char+)' '*)+ | "None")
);

pub fn install_packages<I, S>(packages: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let _pacman_command = Command::new("pacman")
        .arg("-S")
        .args(packages)
        .stdin(Stdio::inherit())
        .status()
        .context("Failed to run pacman")?;
    Ok(())
}

/// Ensures the contained regex is only compiled once to avoid performance impact in loops
/// Thread-safe due to usage of OnceLock
fn re_package_required_by() -> &'static Regex {
    static RE : OnceLock<Regex> = OnceLock::new();
    // TODO: separate test that ensures regex always compiles
    // so this never fails at runtime
    RE.get_or_init(|| Regex::new(RE_PACKAGE_REQUIRED_BY).unwrap())
}

/// Queries what packages are installed on the system
pub fn query_explicitly_installed() -> anyhow::Result<BTreeSet<String>> {
    let pacman_output_bytes = Command::new("pacman")
        .arg("-Qqe")
        .output()
        .context("Failed to run pacman -Qqe")?
        .stdout;
    let pacman_output_string = String::from_utf8(pacman_output_bytes).context("Failed to parse pacman stdout as utf8")?;
    let mut installed_set : BTreeSet<String> = BTreeSet::new();
    for line in pacman_output_string.lines() {
        installed_set.insert(line.to_owned());
        // TODO: could it be more efficient to have a consuming iterator and not have to use to_owned?
    }
    Ok(installed_set)
}

/// Uninstalls the passed packages
pub fn uninstall_packages<I, S>(packages: I) -> anyhow::Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("pacman")
        .arg("-Rs")
        .args(packages)
        .stdin(Stdio::inherit())
        .status().context("Failed to run pacman -Rs")
}

// pub fn package_required_by(package: &str) -> anyhow::Result<Vec<String>> {
//     let pacman_output_bytes = Command::new("pacman")
//         .arg("-Qi")
//         .arg(package)
//         .output()
//         .context("Failed to run pacman -Qi")?
//         .stdout;

//     let pacman_output_bytes = std::str::from_utf8(&pacman_output_bytes);
// }

/// Gets the packages requiring the passed packages.
/// The vec may conain "None" instead of package names,
/// indicating no packages requiring the given package.
/// TODO(low, ergonomics): consider returning hashmap instead (mappign package name -> dependants)
pub fn packages_required_by(packages: Vec<String>) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    let pacman_output_bytes = Command::new("pacman")
        .arg("-Qi")
        .args(&packages)
        .output()
        .context("Failed to run pacman -Qi")?
        .stdout;

    // TODO(low, cross-platform):
    // use OsStr instead - effectively not a problem on linux
    // because strings are utf-8, but might cause issues on other OS if that is ever relevant 
    let pacman_output_str = std::str::from_utf8(&pacman_output_bytes).context("Failed to parse pacman stdout as utf8")?;
    let dependants = parse_required_by_many(pacman_output_str)?;

    Ok(packages.into_iter().zip(dependants).collect())
}
   
fn parse_required_by_many(pacman_output: &str) -> anyhow::Result<Vec<Vec<String>>> {
    
    // TODO(low, optimiztion): get capactiy, init with capacity
    let mut result = Vec::new();

    // separate into the chunks concerning each package
    // skips the first empty part because the data starts with the "delimiter"
    for single_package_chunkj in pacman_output.split("Name").skip(1) {
        let package_dependents = parse_required_by(single_package_chunkj)?;
        result.push(package_dependents);
    }
    Ok(result)
}

/// Parses the packages required
// TODO(medium, ergonomics): check for "None" in "Required By"
// and return option accordingly
fn parse_required_by(pacman_output: &str) -> anyhow::Result<Vec<String>> {
    let re = re_package_required_by();
    let Some(caps) = re.captures(pacman_output) else {
        // TODO: check if this and to be expected if there are no dependants
        // consider also parsing the "None" case
        bail!("No matches for 'Required by'");
    };

    let dependants =
        caps
            .get(1)
            .context("Failed to get capturing group 1")?
            .as_str()
            .split_whitespace()
            .map(|s| s.to_owned())
            .collect();
    Ok(dependants)
}

#[cfg(test)]
mod tests {
    use super::*;

    const pacman_Qi_output_xorg : &str =
r#"Name            : xorg-server
Version         : 21.1.8-2
Description     : Xorg X server
Architecture    : x86_64
URL             : https://xorg.freedesktop.org
Licenses        : custom
Groups          : xorg
Provides        : X-ABI-VIDEODRV_VERSION=25.2  X-ABI-XINPUT_VERSION=24.4  X-ABI-EXTENSION_VERSION=10.0  x-server
Depends On      : libepoxy  libxfont2  pixman  xorg-server-common  libunwind  dbus  libgl  xf86-input-libinput  nettle  libpciaccess  libdrm  libxshmfence
                    libxcvt
Optional Deps   : None
Required By     : lightdm  lightdm-slick-greeter
Optional For    : None
Conflicts With  : nvidia-utils<=331.20  glamor-egl  xf86-video-modesetting
Replaces        : glamor-egl  xf86-video-modesetting
Installed Size  : 3,73 MiB
Packager        : Laurent Carlier <lordheavym@archlinux.org>
Build Date      : Mo 10 Jul 2023 11:25:36 CEST
Install Date    : Mi 12 Jul 2023 17:24:40 CEST
Install Reason  : Explicitly installed
Install Script  : Yes
Validated By    : Signature
"#;

    const pacman_Qi_output_lightdm_slick : &str =
r#"Name            : eos-lightdm-slick-theme
Version         : 3.2-1
Description     : EndeavourOS theme for lightdm-slick-greeter
Architecture    : any
URL             : https://www.endeavouros.com
Licenses        : GPL3
Groups          : None
Provides        : None
Depends On      : lightdm  lightdm-slick-greeter
Optional Deps   : eos-qogir-icons [installed]
                  arc-gtk-theme-eos [installed]
Required By     : None
Optional For    : None
Conflicts With  : None
Replaces        : None
Installed Size  : 382,00 B
Packager        : EndeavourOS <info@endeavouros.com>
Build Date      : Mi 21 Dez 2022 14:48:28 CET
Install Date    : Fr 03 Feb 2023 14:51:35 CET
Install Reason  : Explicitly installed
Install Script  : Yes
Validated By    : Signature
"#;

    #[test]
    fn test_parse_required_by() {
        {
            let parsed_dependants = parse_required_by(pacman_Qi_output_xorg).unwrap();
            let expected = vec!["lightdm", "lightdm-slick-greeter"];
            assert_eq!(expected, parsed_dependants)
        }
        {
            let  parse_dependants = parse_required_by(pacman_Qi_output_lightdm_slick).unwrap();
            let expected = vec!["None"];
            assert_eq!(expected, parse_dependants)
        }
    }
}