mod cli;
mod declareman;


use clap::Parser;
use declareman::{DeclaremanConfig, PackageConfiguration, install_packages, ActiveTarget};
use dialoguer::{Confirm, Editor};
use directories::ProjectDirs;
use std::{path::{PathBuf}, fs::{self, File}, env, io, collections::BTreeSet};
use std::io::Write;
use anyhow::{Context, bail};
use cli::{Cli, CliCommand, Target, Groups};

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

    let config : DeclaremanConfig = {
        let config_string = match fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => {
                    println!("Failed to find declareman config file at {}", config_path.to_str().unwrap_or("<invalid unicode>"));
                    if Confirm::new().with_prompt("Create a new config file?").interact()? {
                        println!("Let's create a new file");

                        // write default values, then open editor
                        let config_template = declareman::DeclaremanConfig::template();
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

        toml::from_str(&config_string).context("Failed to parse declareman configuration")?
    };

    let mut package_config = PackageConfiguration::parse(&config, &config_path).context("Failed to parse package configuration")?;
    
    let mut active_target = match std::fs::read_to_string(&active_target_path) {
        Ok(s) => ActiveTarget::parse(&s).context(format!("Failed to parse active target at '{}'", &active_target_path.to_string_lossy()))?,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => {
                println!("Failed to find active target file at {}", config_path.to_str().unwrap_or("<invalid unicode>"));
                println!("Please select a new active target");

                let targets : Vec<&String> = config.targets.keys().collect();
                let selection = match dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
                active_target
            },
            _ => {
                bail!(err)
            }
        },
    };

    match &cli.command {
        Some(CliCommand::Config) => {
            println!("config: {:?}", config);
        }
        Some(CliCommand::Sync) => {
            install_packages(&package_config.groups, &config.root_group)?;
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
                    for (name, _) in config.targets {
                        if &name == active_target {
                            println!("{} (active)", name);
                        } else {
                            println!("{}", name);
                        }
                    }
                },
                Target::Get => {
                    println!("{}", active_target.target());
                },
                Target::Set { target } => {
                    active_target.set_target(target.clone(), &active_target_path).context("Failed to set active target")?;
                },
            }
        }
        Some(CliCommand::Groups(subcommand)) => {
            match subcommand {
                // TODO: currently only lists installed groups instead of all
                Groups::Ls => {
                    package_config.groups.keys().for_each(|group_name| println!("{}", group_name))
                }
            }
        }
        Some(CliCommand::Diff { untracked }) => {
            let pacman_installed = declareman::pacman_query_installed().context("Failed to query installed packages")?;
            let intersection : BTreeSet<String> = pacman_installed.intersection(&package_config.installed_packages()).cloned().collect();

            // TODO: Consider making this section optional
            println!("Installed on the system");
            for pkg in intersection.iter() {
                println!("{}", pkg)
            }

            let not_installed : BTreeSet<String>= package_config.installed_packages().iter().cloned().filter(|package| !intersection.contains(package)).collect();
            println!("Not installed on the system:");
            for pkg in not_installed {
                println!("{}", &pkg)
            }
            
            if *untracked {
                println!("Untracked packages:");
                let untracked_packages : BTreeSet<String> = pacman_installed.iter().cloned().filter(|package| !intersection.contains(package)).collect();
                for pkg in untracked_packages {
                    println!("{}", &pkg)
                }
            }
        }
        Some(CliCommand::Template) => {
            let system_configuration = declareman::distro::get_system_configuration().context("Failed to get system configuration")?;
            // TODO: switch to a display implementation
            println!("System configuration: {:?}", &system_configuration);
            let new_groups = declareman::distro::generate_configuration(&system_configuration).context("Failed to template packages for your system configuration")?;
            
            // TODO: refactor to separate function
            // TODO: remove unwrap
            let mut file_path = config_path.parent().unwrap()
                .join(&config.package_dir)
                .join(system_configuration.distro);

            file_path.set_extension("toml");

            let mut file = File::create(file_path).context("Failed to create file for new package group")?;
            let toml = toml::ser::to_string_pretty(&new_groups)?;
            write!(file, "{}", toml)?;
        }
        None => {},
    }
    Ok(())
}
