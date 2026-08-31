mod installer;
mod finder;
mod remove;
mod upgrade;

use crate::repo::{find_all_packages, find_package_exact, find_package_in_repo};
use colored::*;
use installer::Installer;
use remove::remove_package;
use std::io::Write;
use std::fs;
use std::path::Path;

// Экспортируем функции
pub use upgrade::upgrade_package;
pub use upgrade::update_all_packages;

pub fn install_package(pkgname: &str) {
    let packages = find_all_packages(pkgname);
    
    if packages.is_empty() {
        println!("\n{}", format!("No packages found for '{}'", pkgname).red());
        return;
    }
    
    println!("\n{}", "Found packages:".bold());
    for (i, pkg) in packages.iter().enumerate() {
        let size = pkg.size.as_deref().unwrap_or("???");
        let license = pkg.license.as_deref().unwrap_or("Unknown");
        println!("({}) {}-{} ({}) [{}] [{}]", 
            i + 1, 
            pkg.name.cyan(), 
            pkg.version.green(), 
            pkg.repo.yellow(), 
            size, 
            license
        );
    }
    
    let selected = if packages.len() > 1 {
        print!("\n{}", format!("Choose repository (1-{}, default 1): ", packages.len()).bold());
        std::io::stdout().flush().unwrap();
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        if choice < 1 || choice > packages.len() { 0 } else { choice - 1 }
    } else {
        0
    };
    
    let pkg = &packages[selected];
    
    println!("\n{}: {}", "Package".bold().green(), pkg.name.cyan());
    println!("{}: {}", "Version".bold().green(), pkg.version.green());
    println!("{}: {}", "Repository".bold().green(), pkg.repo.yellow());
    if let Some(size) = &pkg.size {
        println!("{}: {}", "Size".bold().green(), size);
    }
    if !pkg.dependencies.is_empty() {
        println!("{}:", "Dependencies".bold().green());
        for dep in &pkg.dependencies {
            println!("  - {}", dep);
        }
    }
    
    // Проверяем зависимости
    let mut deps_to_install = Vec::new();
    if !pkg.dependencies.is_empty() {
        println!("\n{}", "Checking dependencies...".bold());
        for dep_name in &pkg.dependencies {
            if let Some(dep_pkg) = find_package_in_repo(dep_name, &pkg.repo) {
                println!("  ✓ {} found in {}", dep_name.green(), dep_pkg.repo);
                deps_to_install.push(dep_pkg);
            } else if let Some(dep_pkg) = find_package_exact(dep_name) {
                println!("  ✓ {} found in {}", dep_name.green(), dep_pkg.repo);
                deps_to_install.push(dep_pkg);
            } else {
                let all = find_all_packages(dep_name);
                if !all.is_empty() {
                    println!("  ✓ {} found in {}", dep_name.green(), all[0].repo);
                    deps_to_install.push(all[0].clone());
                } else {
                    println!("  ✗ {} not found in any repository", dep_name.red());
                }
            }
        }
    }
    
    print!("\n{}", "Proceed with installation? [Y/n]: ".bold());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" && !input.trim().is_empty() {
        println!("{}", "Installation cancelled.".yellow());
        return;
    }
    
    if !deps_to_install.is_empty() {
        println!("\n{}", "Installing dependencies:".bold());
        let installer = Installer::new();
        for dep in &deps_to_install {
            if installer.is_installed(dep) {
                println!("  ✓ {} already installed", dep.name.green());
            } else if let Err(e) = installer.install_package(dep) {
                println!("  ✗ Failed to install {}: {}", dep.name, e);
            }
        }
    }
    
    println!("\n{}", "Installing:".bold());
    let installer = Installer::new();
    
    if installer.is_installed(pkg) {
        println!("  ✓ {} already installed", pkg.name.green());
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let bin_dir = format!("{}/.local/bin", home);
        let install_dir = format!("{}/.citadel/{}/{}", home, pkg.repo, pkg.name);
        let _ = installer.create_symlinks_forced(&install_dir, pkg, &bin_dir);
        println!("\n{}", format!("Done!\n{} already installed.", pkg.name).yellow().bold());
        return;
    }
    
    match installer.install_package(pkg) {
        Ok(_) => {
            println!("\n{}", format!("Done!\n{} installed successfully.", pkg.name).green().bold());
        }
        Err(e) => {
            println!("{}", format!("Installation failed: {}", e).red());
        }
    }
}

pub fn remove_package_cmd(pkgname: &str) {
    remove_package(pkgname);
}

pub fn list_installed_packages() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let citadel_dir = format!("{}/.citadel", home);
    
    if !Path::new(&citadel_dir).exists() {
        println!("{}", "No packages installed".red());
        return;
    }
    
    let mut packages = Vec::new();
    if let Ok(entries) = fs::read_dir(&citadel_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let repo_dir = entry.path();
                if repo_dir.is_dir() {
                    let repo_name = repo_dir.file_name().unwrap().to_string_lossy().to_string();
                    if let Ok(pkg_entries) = fs::read_dir(&repo_dir) {
                        for pkg_entry in pkg_entries {
                            if let Ok(pkg_entry) = pkg_entry {
                                let path = pkg_entry.path();
                                if path.is_dir() {
                                    let pkg_name = path.file_name().unwrap().to_string_lossy().to_string();
                                    packages.push((repo_name.clone(), pkg_name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if packages.is_empty() {
        println!("{}", "No packages installed".red());
        return;
    }
    
    println!("\n{}", format!("Installed packages ({}):", packages.len()).bold().green());
    for (repo, pkg) in packages {
        println!("  {} ({})", pkg.cyan(), repo.yellow());
    }
}
mod sinstall;

pub use sinstall::source_install;

pub fn generate_conf() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = format!("{}/.cit.conf", home);
    
    if std::path::Path::new(&config_path).exists() {
        print!("{}", format!("Config file already exists at {}. Overwrite? [y/N] ", config_path).yellow());
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("{}", "Generation cancelled.".yellow());
            return;
        }
    }
    
    let config = r#"# ==========================================
# Citadel Package Manager Configuration
# ==========================================

# ----- Arch Linux -----
[arch/core]
url = https://mirror.yandex.ru/archlinux/core/os/x86_64
type = arch
enabled = 1

[arch/extra]
url = https://mirror.yandex.ru/archlinux/extra/os/x86_64
type = arch
enabled = 1

[arch/multilib]
url = https://mirror.yandex.ru/archlinux/multilib/os/x86_64
type = arch
enabled = 1

# ----- Debian -----
[debian/stable]
url = http://mirror.yandex.ru/debian
type = deb
enabled = 1
suite = stable

# ----- Devuan (без systemd) -----
[devuan/excalibur]
url = http://deb.devuan.org/merged
type = deb
enabled = 1
suite = excalibur
distro = devuan

# ----- Artix Linux (Arch без systemd) -----
# [artix/openrc]
# url = https://mirror.yandex.ru/artix/repos/openrc
# type = arch
# enabled = 0
# distro = artix

# ----- Void Linux (runit) -----
# [void/current]
# url = https://repo-default.voidlinux.org/current
# type = xbps
# enabled = 0
# distro = void

# ----- Alpine Linux (musl) -----
# [alpine/edge]
# url = http://mirrors.tuna.tsinghua.edu.cn/alpine/edge/main
# type = apk
# enabled = 0
# distro = alpine
"#;
    
    if let Err(e) = std::fs::write(&config_path, config) {
        println!("{}", format!("Failed to write config: {}", e).red());
        return;
    }
    
    println!("{}", format!("✓ Config generated at {}", config_path).green().bold());
}
