use std::{collections::BTreeSet, process::Command};

use anyhow::Context;

pub fn query_installed() -> anyhow::Result<BTreeSet<String>> {
    let pacman_output_bytes = Command::new("pacman")
        .arg("-Qqe")
        .output()
        .context("Failed to run pacman -Qq")?
        .stdout;
    let pacman_output_string = String::from_utf8(pacman_output_bytes).context("Failed to parse pacman stdout as utf8")?;
    let mut installed_set : BTreeSet<String> = BTreeSet::new();
    for line in pacman_output_string.lines() {
        installed_set.insert(line.to_owned());
        // TODO: could it be more efficient to have a consuming iterator and not have to use to_owned?
    }
    Ok(installed_set)
}
