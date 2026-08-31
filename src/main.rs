mod cli;
mod config;
mod repo;
mod package;
mod fetch;
mod extract;
mod install;
mod utils;
mod init;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    
    match args.command {
        cli::Commands::Search { pattern } => {
            repo::search_packages(&pattern);
        }
        cli::Commands::Install { package } => {
            install::install_package(&package);
        }
        cli::Commands::Sinstall { package, repo } => {
            install::source_install(&package, repo.as_deref());
        }
        cli::Commands::Remove { package } => {
            install::remove_package_cmd(&package);
        }
        cli::Commands::Upgrade { package } => {
            install::upgrade_package(&package);
        }
        cli::Commands::Update => {
            install::update_all_packages();
        }
        cli::Commands::List => {
            install::list_installed_packages();
        }
        cli::Commands::GenerateConf => {
            install::generate_conf();
        }
    }
}
