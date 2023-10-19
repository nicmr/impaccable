mod cli;
mod declareman;


use clap::Parser;
use declareman::{install_packages, config::{DeclaremanConfigManager, ActiveTarget}, pacman};
use dialoguer::{Confirm, Editor, theme::ColorfulTheme, Input, FuzzySelect, MultiSelect, Select};
use directories::ProjectDirs;
use std::{path::PathBuf, fs::{self, File}, env, io, collections::BTreeSet};
use std::io::Write;
use anyhow::{Context, bail};
use cli::{Cli, CliCommand, Target, Groups};

use crate::declareman::pacman::packages_required_by;

fn main() -> std::result::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // TODO: replace expect
    // TODO: check if 'directories' crate is even needed, as this only runs on Linux anyway, and its main benefit is being cross-platform
    let default_project_dirs = ProjectDirs::from("dev.nicolasmohr.declareman", "Declareman Devs", "declareman")
    .expect("Failed to compute ProjectDirs");

    let config_path = {
        if let Some(cli_config_override) = cli.config {
            cli_config_override
        }
        else if let Ok(env_config_override) = env::var("DECLAREMAN_CONFIG") {
            PathBuf::from(env_config_override)
        } else {
            let mut default_config_path = default_project_dirs.config_dir().to_path_buf();
            default_config_path.push("config.toml");
            default_config_path
        }
    };

    let active_target_path = {
        if let Some(cli_target_override) = cli.target {
            cli_target_override
        } else if let Ok(env_target_override) = env::var("DECLAREMAN_TARGET") {
            PathBuf::from(env_target_override)
        } else {
            let mut default_target_path = default_project_dirs.config_dir().to_path_buf();
            default_target_path.push("active-target.toml");
            default_target_path
        }
    };



    let config_manager : declareman::config::DeclaremanConfigManager = {
        let config_string = match fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    println!("Failed to find declareman config file at {}", config_path.to_str().unwrap_or("<invalid unicode>"));
                    if Confirm::new().with_prompt("Create a new config file?").interact()? {
                        println!("Let's create a new file");

                        // write default values, then open editor
                        let config_template = declareman::config::DeclaremanConfig::template();
                        let serialized_template = toml::to_string_pretty(&config_template)?;

                        
                        if let Some(custom_config) = Editor::new().extension(".toml").edit(&serialized_template).context("Failed to edit config file template")? {
                            let parent_dirs = &config_path.parent().expect("Failed to extract parent dir of config path");               
                            fs::create_dir_all(parent_dirs)?;
                            let mut file = fs::File::create(&config_path)?;
                            write!(file, "{}", custom_config)?;
                            custom_config
                        } else {
                            bail!(String::from("Config file creation aborted / input not saved"))
                        }
                    }
                    else {
                        bail!(String::from("No config file found and creation of new file declined."))
                    }
                },
                _ => {
                    bail!(err)
                }
            }
        };

        let config = toml::from_str(&config_string).context("Failed to parse declareman configuration")?;
        DeclaremanConfigManager::new(config_path, config)
    };

    let mut package_config = config_manager.package_configuration().context("Failed to parse package configuration")?;
    
    let mut active_target = match std::fs::read_to_string(&active_target_path) {
        Ok(s) => ActiveTarget::parse(&s).context(format!("Failed to parse active target at '{}'", &active_target_path.to_string_lossy()))?,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => {
                println!("Failed to find active target file at {}", &active_target_path.to_string_lossy());
                println!("Please select a new active target");

                let targets : Vec<&String> = config_manager.config().targets.keys().collect();
                let selection = match Select::with_theme(&ColorfulTheme::default())
                    .items(&targets)
                    .interact_opt() {
                        Ok(Some(sel)) => {
                            sel
                        },
                        Ok(None) => {
                            bail!("Target selection aborted")
                        }
                        Err(_err) => {
                            bail!("Target selection crashed")
                        }
                };
                let active_target = ActiveTarget::new(targets[selection].to_string());
                let mut file = File::create(&active_target_path).context("Failed to create file for new package group")?;
                let toml = toml::ser::to_string_pretty(&active_target)?;
                write!(file, "{}", toml)?;
                active_target
            },
            _ => {
                bail!(err)
            }
        },
    };

    match &cli.command {
        Some(CliCommand::Config) => {
            println!("config: {:?}", config_manager.config());
        }
        Some(CliCommand::Sync { remove_untracked }) => {

            let pacman_installed = declareman::pacman::query_explicitly_installed().context("Failed to query installed packages")?;
            // let not_installed_by_group = package_config.not_installed_packages_by_group(&pacman_installed);
            let not_installed : BTreeSet<String> = package_config.packages().iter().cloned().filter(|package| !pacman_installed.contains(package)).collect();
            install_packages(not_installed).context("Failed to install missing packages")?;
            
            if *remove_untracked {
                let untracked_packages = pacman_installed.iter().cloned().filter(|package| package_config.packages().contains(package));
                let _uninstall_exit_status = declareman::pacman::uninstall_packages(untracked_packages)?;
            }
            // install_group_packages(&package_config.groups, &config_manager.config().root_group)?;
        }
        Some(CliCommand::Add { packages, group }) => {
            let unique_packages = packages.clone().into_iter().collect();
            package_config.add_packages(unique_packages, group).context("Failed to add packages")?;
            println!("Added the following packages{:?}", packages);
        }
        Some(CliCommand::Remove { package }) => {
            let removed_from_groups = package_config.remove_package(package);

            println!("Removed from the following groups: {:?}", removed_from_groups)
            // TODO: BUG: currently, remove does not perform a save, this has to be fixed
        }
        Some(CliCommand::Target(subcommand)) => {
            match subcommand {
                Target::Ls => {
                    let active_target = active_target.target();
                    for name in config_manager.config().targets.keys() {
                        if name == active_target {
                            println!("{} (active)", name);
                        } else {
                            println!("{}", name);
                        }
                    }
                },
                Target::Get => {
                    println!("{}", active_target.target());
                },
                Target::Set { target, force } => {
                    if *force || config_manager.config().targets.contains_key(target) {
                        active_target.set_target(target.clone(), &active_target_path).context("Failed to set active target")?;
                    } else {
                        println!("Did not set target '{}' because it is not in the list of available targets. Check targets with `target ls` or override with `--force`", target)
                    }
                },
            }
        }
        Some(CliCommand::Groups(subcommand)) => {
            match subcommand {
                Groups::Ls => {
                    package_config.groups.keys().for_each(|group_name| println!("{}", group_name))
                }
            }
        }
        Some(CliCommand::Diff { untracked }) => {
            let pacman_installed = declareman::pacman::query_explicitly_installed().context("Failed to query installed packages")?;
            let intersection : BTreeSet<String> = pacman_installed.intersection(&package_config.packages()).cloned().collect();

            use colored::Colorize;

            // TODO: Consider making this section optional
            println!("{}", "Installed on the system".blue());
            for pkg in intersection.iter() {
                println!("{}", pkg.blue())
            }

            let not_installed : BTreeSet<String>= package_config.packages().iter().cloned().filter(|package| !intersection.contains(package)).collect();
            println!("{}", "Not installed on the system:".green());
            for pkg in not_installed {
                println!("{}", &pkg.green())
            }
            
            if *untracked {
                println!("Untracked packages:");
                let untracked_packages : BTreeSet<String> = pacman_installed.iter().cloned().filter(|package| !intersection.contains(package)).collect();
                for pkg in untracked_packages {
                    println!("{}", &pkg)
                }
            }
        }
        
        Some(CliCommand::Plan { remove_untracked }) => {
            let pacman_installed = declareman::pacman::query_explicitly_installed().context("Failed to query installed packages")?;
            // let intersection : BTreeSet<String> = pacman_installed.intersection(&package_config.packages()).cloned().collect();

            // let intersection_by_group = declareman::group_intersection(package_config.groups, &pacman_installed);

            let not_installed_by_group = package_config.not_installed_packages_by_group(&pacman_installed);

            use colored::Colorize;

            println!("{}", "Sync would install the following programs");

            for (group, missing_packages) in not_installed_by_group {
                if !missing_packages.members.is_empty() {
                    println!("{}", format!("From group '{}':", group).green());
                    for pkg in missing_packages.members {
                        println!("{} {}", "+".green(), &pkg.green())
                    }
                }
            }

            if *remove_untracked {
                println!("{}", "sync --remove-untracked would remove the following programs");

                // TODO(low, optimization): prevent excessive Vec and String allocations
                let untracked_packages : Vec<String> = pacman_installed.iter().cloned().filter(|package| !package_config.packages().contains(package)).collect();

                for (untracked_package, required_by) in packages_required_by(untracked_packages)? {
                    if required_by[0] == "None" {
                        println!("{} {}", "-".red(), untracked_package.red() )
                    }
                }
            }
        }

        Some(CliCommand::Template) => {
            let system_configuration = declareman::distro::get_system_configuration().context("Failed to get system configuration")?;
            // TODO: switch to a display implementation
            println!("System configuration: {:?}", &system_configuration);
            let new_groups = declareman::distro::generate_configuration(&system_configuration).context("Failed to template packages for your system configuration")?;
            
            // TODO: handle absolute path
            let mut file_path = config_manager.absolute_package_dir()?;
            file_path.push(system_configuration.distro);
            file_path.set_extension("toml");

            let mut file = File::create(file_path).context("Failed to create file for new package group")?;
            let toml = toml::ser::to_string_pretty(&new_groups)?;
            write!(file, "{}", toml)?;
        }
        None => {},

        Some(CliCommand::Import) => {
            let pacman_installed = declareman::pacman::query_explicitly_installed().context("Failed to query installed packages")?;
            // TODO(low, optimization): prevent excessive Vec and String allocations
            let untracked_packages : Vec<String> = pacman_installed.iter().cloned().filter(|package| !package_config.packages().contains(package)).collect();

            let multi_selection = match MultiSelect::with_theme(&ColorfulTheme::default())
                // BUG(low, ux, upstream?): prompt only shows on second page if paginated
                // check if bug is fixable or provide dialog beforehand explaining what to do
                .with_prompt("Select the packages you would like to import into your configuration")
                .items(&untracked_packages)
                .interact_opt() {
                    Ok(Some(sel)) => sel,
                    Ok(None) => {
                        bail!("Package selection aborted")
                    },
                    Err(_err) => {
                        bail!("Package selection crashed")
                    },
                };
            if multi_selection.len() < 1 {
                return Ok(());
            }

            let groups: Vec<&String> = package_config.groups.keys().collect();

            let group_selection = match FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select the group to add the packages to")
                .item("New group")
                .items(&groups)
                .interact_opt() {
                    Ok(Some(sel)) => sel,
                    Ok(None) => {
                        bail!("Group selection aborted")
                    },
                    Err(_err) => {
                        bail!("Group selection crashed")
                    },
                };

            // Check if new group was selected
            if group_selection == 0 {
                // create group
                let new_group_name: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Name for new group")
                    // TODO(low, limitation): only ascii characters allowed by interact_text
                    .interact_text().context("Failed to get new group name")?;
                
                println!("{}", new_group_name);
            } else {
                // requires decrement by 1 because new group item is prepended
                println!("{}", groups[group_selection-1])
            }
            // add packages to group
        }
    }
    Ok(())
}

