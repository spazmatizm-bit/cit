use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use crate::repo::{find_package_exact, find_package_in_repo};

fn find_packages_recursive(dir: &Path, found: &mut Vec<(String, std::path::PathBuf, String)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    
                    let has_pkginfo = path.join(".PKGINFO").exists();
                    let has_debian_control = path.join("DEBIAN/control").exists();
                    let has_usr_bin = path.join("usr/bin").exists();
                    let has_bin = path.join("bin").exists();
                    
                    if has_pkginfo || has_debian_control || has_usr_bin || has_bin {
                        let repo_name = if let Some(parent) = path.parent() {
                            let parent_name = parent.file_name().unwrap().to_string_lossy().to_string();
                            let grandparent = if let Some(gp) = parent.parent() {
                                gp.file_name().unwrap().to_string_lossy().to_string()
                            } else {
                                String::new()
                            };
                            if !grandparent.is_empty() && grandparent != ".citadel" {
                                format!("{}/{}", grandparent, parent_name)
                            } else {
                                parent_name
                            }
                        } else {
                            "unknown".to_string()
                        };
                        found.push((repo_name, path, name));
                    } else {
                        find_packages_recursive(&path, found);
                    }
                }
            }
        }
    }
}

pub fn upgrade_package(pkgname: &str) {
    println!("\n{}", format!("Upgrading {}...", pkgname).bold().yellow());
    
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let citadel_dir = format!("{}/.citadel", home);
    
    if !Path::new(&citadel_dir).exists() {
        println!("{}", "No packages installed".red());
        return;
    }
    
    let mut all_packages = Vec::new();
    find_packages_recursive(Path::new(&citadel_dir), &mut all_packages);
    
    let found_packages: Vec<_> = all_packages
        .iter()
        .filter(|(_, _, name)| {
            name == pkgname || 
            name.starts_with(&format!("{}-", pkgname)) ||
            name.starts_with(&format!("{}_", pkgname))
        })
        .cloned()
        .collect();
    
    if found_packages.is_empty() {
        println!("{}", format!("Package '{}' is not installed", pkgname).red());
        println!("\n{}", "Installed packages:".bold());
        for (repo, _, name) in &all_packages {
            println!("  {} ({})", name.cyan(), repo.yellow());
        }
        return;
    }
    
    let (repo, path, full_name) = if found_packages.len() > 1 {
        println!("\n{}", "Found multiple copies:".bold());
        for (i, (r, _, n)) in found_packages.iter().enumerate() {
            println!("  ({}) {} (repository: {})", i + 1, n, r);
        }
        
        print!("\n{}", format!("Choose which one to upgrade (1-{}, default 1): ", found_packages.len()).bold());
        std::io::stdout().flush().unwrap();
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        let idx = if choice < 1 || choice > found_packages.len() { 0 } else { choice - 1 };
        found_packages[idx].clone()
    } else {
        found_packages[0].clone()
    };
    
    let clean_name = if full_name.contains('-') {
        full_name.split('-').next().unwrap_or(&full_name).to_string()
    } else {
        full_name.clone()
    };
    
    println!("\n{}", format!("Upgrading {} from {}...", full_name.cyan(), repo.yellow()));
    
    let current_version = if full_name.contains('-') {
        full_name.split('-').last().unwrap_or("unknown")
    } else {
        "unknown"
    };
    println!("  Current version: {}", current_version.red());
    
    // Ищем новую версию ТОЛЬКО в том же репозитории
    let new_pkg = match find_package_in_repo(&clean_name, &repo) {
        Some(pkg) => pkg,
        None => {
            println!("  ℹ No newer version found in same repository");
            println!("  ✓ {} is up to date", full_name.green());
            return;
        }
    };
    
    // Проверяем, что версия действительно новее
    if current_version == new_pkg.version {
        println!("  ✓ {} is already up to date", full_name.green());
        return;
    }
    
    println!("  New version: {}", new_pkg.version.green());
    
    print!("\n{}", "Proceed with upgrade? [Y/n] ".bold());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" && !input.trim().is_empty() {
        println!("{}", "Upgrade cancelled.".yellow());
        return;
    }
    
    println!("  → Removing old version...");
    if let Err(e) = fs::remove_dir_all(&path) {
        println!("  ✗ Failed to remove old version: {}", e);
        return;
    }
    
    let bin_dir = format!("{}/.local/bin", home);
    if Path::new(&bin_dir).exists() {
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let link_path = entry.path();
                    if let Ok(target) = fs::read_link(&link_path) {
                        if target.starts_with(&path) {
                            let _ = fs::remove_file(&link_path);
                        }
                    }
                }
            }
        }
    }
    
    println!("  → Installing new version...");
    use crate::install::installer::Installer;
    let installer = Installer::new();
    if let Err(e) = installer.install_package(&new_pkg) {
        println!("  ✗ Failed to install new version: {}", e);
        return;
    }
    
    println!("\n{}", format!("✓ {} upgraded to {}", clean_name, new_pkg.version).green().bold());
}

pub fn update_all_packages() {
    println!("\n{}", "Updating all packages...".bold().yellow());
    
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let citadel_dir = format!("{}/.citadel", home);
    
    if !Path::new(&citadel_dir).exists() {
        println!("{}", "No packages installed".red());
        return;
    }
    
    let mut packages = Vec::new();
    find_packages_recursive(Path::new(&citadel_dir), &mut packages);
    
    if packages.is_empty() {
        println!("{}", "No packages installed".red());
        return;
    }
    
    let real_packages: Vec<_> = packages
        .iter()
        .filter(|(_, _, name)| {
            !name.starts_with("lib32-") &&
            !name.starts_with("lib") &&
            !name.starts_with("python-") &&
            !name.contains("dev") &&
            !name.contains("doc") &&
            !name.contains("data")
        })
        .cloned()
        .collect();
    
    if real_packages.is_empty() {
        println!("{}", "No user packages to update".yellow());
        return;
    }
    
    println!("{}", format!("Found {} packages:", real_packages.len()).bold());
    for (i, (repo, _, name)) in real_packages.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, name, repo);
    }
    
    print!("\n{}", "Update these packages? [y/N] ".bold());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" {
        println!("{}", "Update cancelled.".yellow());
        return;
    }
    
    let mut updated = 0;
    for (_, _, name) in &real_packages {
        let clean_name = name.split('-').next().unwrap_or(name);
        println!("\n  → Upgrading {}...", clean_name);
        upgrade_package(clean_name);
        updated += 1;
    }
    
    println!("\n{}", format!("✓ Updated {} packages!", updated).green().bold());
}
