use std::{collections::BTreeSet, process::{Command, Stdio, ExitStatus}, ffi::OsStr, sync::OnceLock};

use anyhow::{Context, bail};
use pomsky_macro::pomsky;
use regex::Regex;

const RE_PACKAGE_REQUIRED_BY: &str = pomsky!(
    let package_name_char = ['a'-'z' '0'-'9' '@' '.' '_' '+' '-'];
    "Required By"[s]+": ":(((package_name_char+)' '*)+ | "None")
);

/// Ensures the contained regex is only compiled once to avoid performance impact in loops.
/// Thread-safe due to usage of OnceLock
fn re_package_required_by() -> &'static Regex {
    static RE : OnceLock<Regex> = OnceLock::new();
    // There's a test ensuring this unwrap never panics: tests::test_required_by_regex_valid
    RE.get_or_init(|| Regex::new(RE_PACKAGE_REQUIRED_BY).unwrap())
}

/// Queries what packages are installed on the system
pub fn query_installed(explicit: bool) -> anyhow::Result<BTreeSet<String>> {
    let mut pacman_args = String::from("-Qq");
    if explicit {
        pacman_args.push('e')
    }
    let pacman_output_bytes = Command::new("pacman")
        .arg(&pacman_args)
        .output()
        .context("Failed to run pacman -Qqe")?
        .stdout;
    let pacman_output_string = String::from_utf8(pacman_output_bytes).context("Failed to parse pacman stdout as utf8")?;
    let mut installed_set : BTreeSet<String> = BTreeSet::new();
    for line in pacman_output_string.lines() {
        installed_set.insert(line.to_owned());
    }
    Ok(installed_set)
}


/// Installs the supplied packages.
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


/// Uninstalls the supplied packages.
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

/// Gets the packages requiring the passed packages.
/// indicating no packages requiring the given package.
/// TODO(low, api): consider returning map instead (mapping package name -> dependants)
pub fn packages_required_by(packages: Vec<String>) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    let pacman_output_bytes = Command::new("pacman")
        .arg("-Qi")
        .args(&packages)
        .output()
        .context("Failed to run pacman -Qi")?
        .stdout;

    // TODO(low, cross-platform): use OsStr instead - effectively not a problem on linux because strings are utf-8, but more idiomatic
    let pacman_output_str = std::str::from_utf8(&pacman_output_bytes).context("Failed to parse pacman stdout as utf8")?;
    let dependants = parse_required_by_many(pacman_output_str, Some(packages.len()))?;

    Ok(packages.into_iter().zip(dependants).collect())
}

/// Parses the `Required By` attribute of the pacman output for many packages.
/// Optionally takes the package count for vec init optimization
fn parse_required_by_many(pacman_output: &str, opt_package_count: Option<usize>) -> anyhow::Result<Vec<Vec<String>>> {
    let mut result = Vec::with_capacity(opt_package_count.unwrap_or(0));

    // separate into the chunks for each package
    // skips the first empty part because the data starts with the "delimiter"
    for single_package_chunkj in pacman_output.split("Name").skip(1) {
        let package_dependents = parse_required_by(single_package_chunkj)?;
        result.push(package_dependents);
    }
    Ok(result)
}

/// Parses the `Required By` attribute of the pacman output for a single package.
/// Packages with a "None" value will return and empty Vec.
fn parse_required_by(pacman_output: &str) -> anyhow::Result<Vec<String>> {
    let re = re_package_required_by();
    let Some(caps) = re.captures(pacman_output) else {
        bail!("No matches for 'Required by'");
    };

    let dependants : Vec<String> =
        caps
            .get(1)
            .context("Failed to get capturing group 1")?
            .as_str()
            .split_whitespace()
            .map(|s| s.to_owned())
            .collect();
    
    // Packages without dependencies have a "None" value.
    if let Some(first_dep) = dependants.get(0) {
        if first_dep == "None" {
            return Ok(Vec::new())
        }
    }

    Ok(dependants)
}

#[cfg(test)]
mod tests {
    use super::*;

    // these become hard to read as all caps, only used in tests
    #[allow(non_upper_case_globals)]
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

    #[allow(non_upper_case_globals)]
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
    fn test_required_by_regex_valid() {
        Regex::new(RE_PACKAGE_REQUIRED_BY).unwrap();
    }

    #[test]
    fn test_parse_required_by() {
        {
            let parsed_dependants = parse_required_by(pacman_Qi_output_xorg).unwrap();
            let expected = vec!["lightdm", "lightdm-slick-greeter"];
            assert_eq!(expected, parsed_dependants)
        }
        {
            let  parse_dependants = parse_required_by(pacman_Qi_output_lightdm_slick).unwrap();
            let expected : Vec<String> = Vec::new();
            assert_eq!(expected, parse_dependants)
        }
    }
}