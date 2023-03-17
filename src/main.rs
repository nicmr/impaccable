mod cli;
mod declareman;


use clap::Parser;
use declareman::{DeclaremanConfig, PackageConfiguration, install_packages, ActiveTarget};
use dialoguer::{Input, Confirm, Editor};
use directories::ProjectDirs;
use std::{path::{Path, PathBuf}, fs, env, io};
use std::io::Write;
use anyhow::{Context, bail};
use cli::{Cli, CliCommand, Target};

fn main() -> std::result::Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let config_path = {
        if let Some(cli_config_override) = cli.config {
            cli_config_override
        }
        else if let Ok(env_config_override)= env::var("DECLAREMAN_CONFIG") {
            PathBuf::from(env_config_override)
        } else {
            // TODO: replace unwrap
            // TODO: check if directories is even needed, as this only runs on Linux anyway, and its main benefit is being cross-platform
            let default_project_dirs = ProjectDirs::from("dev.nicolasmohr.declareman", "Declareman Devs", "declareman")
                .expect("Failed to compute ProjectDirs");
            let mut default_config_path = default_project_dirs.config_dir().to_path_buf();
            default_config_path.push("config.toml");
            default_config_path
        }
    };

    // let config_path = Path::new("./declareman/config.toml");

    let config : DeclaremanConfig = {
        // let config_string = fs::read_to_string(&config_path)
        //     .context(format!("Failed to open declareman config file at {}", config_path.to_str().unwrap_or("invalid unicode")))?;

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

        // handle no config file found: offer to create one
        

        toml::from_str(&config_string).context("Failed to parse declareman configuration")?
    };
    let mut package_config = PackageConfiguration::parse(&config, &config_path).context("Failed to parse package configuration")?;
    
    let active_target_path = Path::new("./declareman/active-target.toml");
    let mut active_target = ActiveTarget::parse(active_target_path).context("Failed to parse active target")?;

    match &cli.command {
        Some(CliCommand::Config) => {
            println!("config: {:?}", config);
        },
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

            // TODO: currently, remove does not perform a save, this has to be fixed
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
                    active_target.set_target(target.clone(), active_target_path).context("Failed to set active target")?;
                },
            }
        }
        Some(CliCommand::Diff) => {
            let pacman_installed = declareman::pacman_query_installed().context("Failed to query installed packages")?;
            for entry in pacman_installed {
                println!("{}", entry)
            }
        }
        None => {},
    }
    Ok(())
}




