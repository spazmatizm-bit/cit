use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "cit")]
#[command(about = "Citadel Package Manager - Universal package manager")]
#[command(version = "0.1.0")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for packages
    Search {
        /// Search pattern
        pattern: String,
    },
    
    /// Install a binary package
    Install {
        /// Package name
        package: String,
    },
    
    /// Install from source (source install)
    Sinstall {
        /// Package name
        package: String,
        
        /// Repository to use (optional)
        #[arg(short, long)]
        repo: Option<String>,
    },
    
    /// Remove a package
    Remove {
        /// Package name
        package: String,
    },
    
    /// Upgrade a single package
    Upgrade {
        /// Package name
        package: String,
    },
    
    /// Update all packages
    Update,
    
    /// List installed packages
    List,
    
    /// Generate default configuration file
    GenerateConf,
}
