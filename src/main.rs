mod cli;
mod impaccable;


use clap::Parser;
use impaccable::{config::{ConfigManager, ActiveTarget}, pacman, PackageId};
use dialoguer::{Confirm, Editor, theme::ColorfulTheme, Input, FuzzySelect, MultiSelect, Select};
use directories::ProjectDirs;
use std::{path::PathBuf, fs::{self, File}, env, io, collections::BTreeSet};
use std::io::Write;
use anyhow::{Context, bail, anyhow};
use cli::{Cli, CliCommand, Target, Groups};

use crate::impaccable::{pacman::packages_required_by, PackageGroup, GroupId};

fn main() -> std::result::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // TODO(low, dependency): check if 'directories' crate is even needed, as this only runs on Linux anyway,
    // and its main benefit over just an xdg crate is being cross-platform
    let default_project_dirs = ProjectDirs::from("dev.nicolasmohr.impaccable", "impaccable devs", "impaccable")
    .context("Failed to compute ProjectDirs")?;

    let active_target_path = {
        if let Some(cli_target_override) = cli.target {
            cli_target_override
        } else if let Ok(env_target_override) = env::var("IMPACCABLE_TARGET") {
            PathBuf::from(env_target_override)
        } else {
            let mut default_target_path = default_project_dirs.config_dir().to_path_buf();
            default_target_path.push("active-target.toml");
            default_target_path
        }
    };

    let mut config_manager : impaccable::config::ConfigManager = {
        let config_path = {
            if let Some(cli_config_override) = cli.config {
                cli_config_override
            }
            else if let Ok(env_config_override) = env::var("IMPACCABLE_CONFIG") {
                PathBuf::from(env_config_override)
            } else {
                let mut default_config_path = default_project_dirs.config_dir().to_path_buf();
                default_config_path.push("config.toml");
                default_config_path
            }
        };

        // Parse the config file. If it is not found, offer to create it instead.
        let config_manager = match ConfigManager::parse(config_path.clone()) {
            Ok(config_manager) => config_manager,
            Err(err) => {
                match err {
                    impaccable::Error::ConfigFileNotFound { path: _, source: _ } => {
                        println!("{}", err);
                        if Confirm::new().with_prompt("Create a new config file?").interact()? {

                            // create template, then offer user to edit
                            let config_template = impaccable::config::Config::template().context("Failed ot create config template contents")?;
                            let serialized_template = toml::to_string_pretty(&config_template)?;
                            
                            if let Some(custom_config) = Editor::new().extension(".toml").edit(&serialized_template).context("Failed to edit config file template")? {
                                let parent_dirs = &config_path.parent().expect("Failed to extract parent dir of config path");               
                                fs::create_dir_all(parent_dirs)?;
                                let mut file = fs::File::create(&config_path)?;
                                write!(file, "{}", custom_config)?;

                                // Now, we can try to parse again
                                ConfigManager::parse(config_path)?
                            } else {
                                bail!(String::from("Config file creation aborted / input not saved"))
                            }
                        } else {
                            bail!(String::from("No config file found and creation of new file declined."))
                        }
                    },
                    _ => bail!(err),
                }
            },
        };
        config_manager
    };
    
    let mut active_target = match std::fs::read_to_string(&active_target_path) {
        Ok(s) => ActiveTarget::parse(&s).context(format!("Failed to parse active target at '{}'", &active_target_path.to_string_lossy()))?,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => {
                println!("Failed to find active target file at {}", &active_target_path.to_string_lossy());
                println!("Please select a new active target");

                let targets : Vec<&String> = config_manager.config().targets.keys().collect();
                let Some(selection) = Select::with_theme(&ColorfulTheme::default())
                    .items(&targets)
                    .interact_opt()
                    .context("Target selection crashed")? else  {
                        bail!("Target selection aborted")
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

    // The following code handles the different CLI (sub)commands, then exits.
    match &cli.command {
        None => {},
        Some(CliCommand::Config) => {
            println!("config: {:?}", config_manager.config());
        }
        Some(CliCommand::Sync { remove_untracked }) => {
            let pacman_installed = impaccable::pacman::query_explicitly_installed().context("Failed to query installed packages")?;

            let target_config = config_manager.config().targets.get(active_target.target()).ok_or_else(|| anyhow!(impaccable::Error::TargetNotFound(active_target.target().clone())))?;
            let should_be_installed : BTreeSet<&PackageId> = config_manager.package_config().packages_of_groups(&target_config.root_groups).collect();

            let not_installed = should_be_installed.iter().filter(|package| !pacman_installed.contains(**package));
            pacman::install_packages(not_installed).context("Failed to install missing packages")?;
            
            if *remove_untracked {
                let untracked_packages = pacman_installed.iter().cloned().filter(|package| should_be_installed.contains(package));
                let _uninstall_exit_status = pacman::uninstall_packages(untracked_packages)?;
            }
        }
        Some(CliCommand::Add { packages, group }) => {
            let unique_packages : BTreeSet<PackageId> = packages.clone().into_iter().collect();
            config_manager.package_config_mut().add_packages(unique_packages, group).context("Failed to add packages")?;
            println!("Added the following packages{:?}", packages);
        }
        Some(CliCommand::Remove { package, group }) => {
            config_manager.package_config_mut().remove_package(package, group)?
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
                    config_manager.package_config().iter_groups().for_each(|(group_name, _)| println!("{}", group_name))
                }
            }
        }
        Some(CliCommand::Plan { remove_untracked }) => {
            let pacman_installed = impaccable::pacman::query_explicitly_installed().context("Failed to query installed packages")?;

            let target = config_manager.config().targets.get(active_target.target()).context(format!("Failed to find root group {} in config", active_target.target()))?;

            println!("Active target: {}", active_target.target());
            println!("Configured groups: {}", toml::to_string(target)?);


            let missing_on_system : Vec<(&GroupId, PackageGroup)> =
                config_manager.package_config().groups(&target.root_groups)
                    .map(|(name, package_contents)|{
                        let not_installed : BTreeSet<PackageId> = package_contents.members.clone()
                            .into_iter()
                            .filter(|package| !pacman_installed.contains(package)).collect();
                        (name, PackageGroup::from_members(not_installed))
                    })
                    .collect();
                
            use colored::Colorize;

            println!("Sync would install the following programs:");

            for (group, missing_packages) in missing_on_system {
                // only display groups that have missing packages
                if !missing_packages.members.is_empty() {
                    println!("{}", format!("From group '{}':", group).green());
                    for pkg in missing_packages.members {
                        println!("{} {}", "+".green(), &pkg.green())
                    }
                }
            }

            if *remove_untracked {
                println!("sync --remove-untracked would remove the following programs:");

                let should_be_installed : BTreeSet<&PackageId> = config_manager.package_config().packages_of_groups(&target.root_groups).collect();
                let untracked_packages : Vec<String> = pacman_installed.iter().cloned().filter(|package| !should_be_installed.contains(package)).collect();

                for (untracked_package, required_by) in packages_required_by(untracked_packages)? {
                    if required_by.is_empty() {
                        println!("{} {}", "-".red(), untracked_package.red() )
                    }
                }
            }
        }

        Some(CliCommand::Template) => {
            let system_configuration = impaccable::distro::get_system_configuration().context("Failed to get system configuration")?;
            // TODO: switch to a display implementation
            println!("Your system configuration: {}", &system_configuration);
            let new_groups = impaccable::distro::generate_configuration(&system_configuration).context("Failed to template packages for your system configuration")?;
            
            let mut file_path = config_manager.absolute_package_dir()?;
            file_path.push(system_configuration.distro);
            file_path.set_extension("toml");

            let mut file = File::create(file_path).context("Failed to create file for new package group")?;
            let toml = toml::ser::to_string_pretty(&new_groups)?;
            write!(file, "{}", toml)?;
        }

        Some(CliCommand::Import) => {
            let pacman_installed = impaccable::pacman::query_explicitly_installed().context("Failed to query installed packages")?;
            
            let target = config_manager
                .config()
                .targets.get(active_target.target())
                .context(format!("Failed to find active target '{}' in config", active_target.target()))?
                .clone();

            let should_be_installed : BTreeSet<&PackageId> = config_manager.package_config().packages_of_groups(&target.root_groups).collect();

            let untracked_packages : Vec<String> = pacman_installed.iter().cloned().filter(|package| !should_be_installed.contains(package)).collect();

            let Some(selected_package_indices) = MultiSelect::with_theme(&ColorfulTheme::default())
                // BUG(low, ux, upstream?): prompt only shows on second page if paginated
                // check if bug is fixable or provide dialog beforehand explaining what to do
                .with_prompt("Select the packages you would like to import into your configuration")
                .items(&untracked_packages)
                .interact_opt()
                .context("Package selection aborted")? else {
                    bail!("Package selection aborted")
            };

            if selected_package_indices.is_empty() {
                return Ok(());
            }

            let groups: Vec<&String> = config_manager.package_config().iter_groups().map(|(name, _)| name).collect();

            let dialogue_theme = ColorfulTheme::default();

            let Some(group_selection) = FuzzySelect::with_theme(&dialogue_theme)
                .with_prompt("Select the group to add the packages to")
                .item("New group")
                .items(&groups)
                .interact_opt()
                .context("File selection crashed")? else {
                    bail!("File selection aborted")
                };

            let group_id = 'group_sel: {

                // Check if new group creation requested or can just use selected group
                if group_selection != 0 {
                    // requires decrement by 1 because "new group" item was prepended
                    break 'group_sel groups[group_selection-1].to_string();
                }

                let new_group_name: String = Input::with_theme(&dialogue_theme)
                    .with_prompt("Name for new group")
                    // TODO(low, limitation): only ascii characters allowed by interact_text
                    .interact_text().context("Failed to get new group name")?;

                let file_paths : Vec<_> = config_manager.package_config().files.keys().collect();
                let file_strs : Vec<_> = file_paths.iter().map(|path| path.to_string_lossy()).collect();

                let Some(file_selection) = FuzzySelect::with_theme(&dialogue_theme)
                    .with_prompt("Select the file to store the group in")
                    .item("New file")
                    .items(&file_strs)
                    .interact_opt()
                    .context("File selection crashed")? else {
                        bail!("File selection aborted")
                    };
                
                let file_path = {

                    // Check if new file creation was selected or can just use selected file
                    if file_selection != 0 {
                        // requires decrement by 1 because "new file" item was prepended
                        file_paths[file_selection-1].clone()
                    } else {
                        let new_file_name_rel : String = Input::with_theme(&dialogue_theme)
                            .with_prompt(format!("New file name (stored inside package directory at '{}')", config_manager.absolute_package_dir()?.to_string_lossy()))
                            .interact_text()
                            .context("Failed to get new file name")?;


                        let new_file_path = {
                            let mut p = config_manager.absolute_package_dir()?; 
                            p.push(new_file_name_rel);
                            p.set_extension("toml");
                            p
                        };
                        config_manager.package_config_mut().create_file(&new_file_path, None)?;
                        new_file_path
                    }
                };

                config_manager.package_config_mut().create_group(new_group_name.clone(), &file_path)?;

                new_group_name
            };

            let selected_packages: BTreeSet<String> = 
                selected_package_indices
                    .iter()
                    .map(|index| untracked_packages[*index].clone())
                    .collect();
                config_manager.package_config_mut().add_packages(selected_packages, &group_id).context("Failed to add packages")?;

            if !target.root_groups.contains(&group_id) {
                let confirmation = Confirm::with_theme(&dialogue_theme)
                    .with_prompt("The selected group is currently inactive. Add to active target's root groups?")
                    .interact()
                    .context("Confirmation aborted")?;
                if confirmation {
                    config_manager.add_root_group(active_target.target(), group_id)?;
                }
            }
        }
    }
    Ok(())
}
