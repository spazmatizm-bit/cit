use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;

// Рекурсивно ищем пакеты в ~/.citadel/
fn find_packages_recursive(dir: &Path, depth: usize, found: &mut Vec<(String, std::path::PathBuf, String)>) {
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
                        find_packages_recursive(&path, depth + 1, found);
                    }
                }
            }
        }
    }
}

// Получить зависимости пакета (из .PKGINFO или DEBIAN/control)
fn get_dependencies(pkg_path: &Path) -> Vec<String> {
    let mut deps = Vec::new();
    
    // Проверяем .PKGINFO (Arch)
    let pkginfo_path = pkg_path.join(".PKGINFO");
    if pkginfo_path.exists() {
        if let Ok(content) = fs::read_to_string(&pkginfo_path) {
            for line in content.lines() {
                if line.starts_with("depend = ") {
                    let dep = line[9..].trim();
                    let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                    deps.push(dep_name.to_string());
                }
            }
        }
    }
    
    // Проверяем DEBIAN/control (Debian)
    let control_path = pkg_path.join("DEBIAN/control");
    if control_path.exists() {
        if let Ok(content) = fs::read_to_string(&control_path) {
            for line in content.lines() {
                if line.starts_with("Depends: ") {
                    let deps_str = line[9..].trim();
                    for dep in deps_str.split(',').map(|s| s.trim()) {
                        let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                        if !dep_name.is_empty() && dep_name != "libc" && !dep_name.starts_with("$") {
                            deps.push(dep_name.to_string());
                        }
                    }
                }
            }
        }
    }
    
    deps
}

pub fn remove_package(pkgname: &str) {
    println!("\n{}", format!("Removing {}...", pkgname).bold().yellow());
    
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let citadel_dir = format!("{}/.citadel", home);
    
    if !Path::new(&citadel_dir).exists() {
        println!("{}", "No packages installed (citadel directory not found)".red());
        return;
    }
    
    // Ищем все пакеты рекурсивно
    let mut all_packages = Vec::new();
    find_packages_recursive(Path::new(&citadel_dir), 0, &mut all_packages);
    
    // Фильтруем по имени пакета
    let found_packages: Vec<_> = all_packages
        .iter()
        .filter(|(_, _, pkg_name)| {
            pkg_name == pkgname || 
            pkg_name.starts_with(&format!("{}-", pkgname)) ||
            pkg_name.starts_with(&format!("{}_", pkgname)) ||
            pkg_name.contains(pkgname)
        })
        .cloned()
        .collect();
    
    if found_packages.is_empty() {
        println!("{}", format!("Package '{}' is not installed", pkgname).red());
        println!("\n{}", "Installed packages:".bold());
        
        let mut tree: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (repo, _, pkg) in &all_packages {
            tree.entry(repo.clone()).or_insert_with(Vec::new).push(pkg.clone());
        }
        
        for (repo, pkgs) in tree.iter() {
            println!("  {}:", repo.cyan().bold());
            for pkg in pkgs {
                println!("    - {}", pkg.green());
            }
        }
        return;
    }
    
    let (repo_name, pkg_path, pkg_full_name) = if found_packages.len() > 1 {
        println!("\n{}", "Found multiple copies:".bold());
        for (i, (repo, _, name)) in found_packages.iter().enumerate() {
            println!("  ({}) {} (repository: {})", i + 1, name, repo);
        }
        
        print!("\n{}", format!("Choose which one to remove (1-{}, default 1): ", found_packages.len()).bold());
        std::io::stdout().flush().unwrap();
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        let idx = if choice < 1 || choice > found_packages.len() { 0 } else { choice - 1 };
        found_packages[idx].clone()
    } else {
        found_packages[0].clone()
    };
    
    println!("\n{}", "Package details:".bold().green());
    println!("  Name: {}", pkg_full_name.cyan());
    println!("  Repository: {}", repo_name.yellow());
    println!("  Location: {}", pkg_path.display());
    
    // Получаем зависимости этого пакета
    let dependencies = get_dependencies(&pkg_path);
    
    // Находим пакеты, которые зависят от этого
    let mut dependents = Vec::new();
    for (_, path, name) in &all_packages {
        if path == &pkg_path { continue; }
        let deps = get_dependencies(path);
        for dep in deps {
            if dep == pkg_full_name || dep == pkgname {
                dependents.push((path.clone(), name.clone()));
                break;
            }
        }
    }
    
    // Если есть зависимые пакеты — предупреждаем
    if !dependents.is_empty() {
        println!("\n{}", format!("⚠️  Found {} packages that depend on '{}':", dependents.len(), pkg_full_name).yellow());
        for (_, name) in &dependents {
            println!("  - {}", name);
        }
        print!("\n{}", "Remove them too? [y/N] ".bold());
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() == "y" {
            for (path, name) in dependents {
                println!("  Removing {}...", name);
                let _ = fs::remove_dir_all(&path);
            }
        } else {
            println!("{}", "Dependent packages will NOT be removed.".yellow());
            print!("\n{}", "Remove anyway? [y/N] ".bold());
            std::io::stdout().flush().unwrap();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            
            if input.trim().to_lowercase() != "y" {
                println!("{}", "Removal cancelled.".yellow());
                return;
            }
        }
    }
    
    // Проверяем зависимости — можно ли их удалить?
    let mut deps_to_remove = Vec::new();
    for dep_name in &dependencies {
        // Проверяем, есть ли другие пакеты, которые используют эту зависимость
        let mut is_needed = false;
        for (_, path, name) in &all_packages {
            if path == &pkg_path { continue; }
            let deps = get_dependencies(path);
            for dep in deps {
                if dep == *dep_name {
                    is_needed = true;
                    break;
                }
            }
            if is_needed { break; }
        }
        
        if !is_needed {
            // Находим сам пакет зависимости
            for (_, path, name) in &all_packages {
                if name == dep_name || name.starts_with(&format!("{}-", dep_name)) {
                    deps_to_remove.push((path.clone(), name.clone()));
                    break;
                }
            }
        }
    }
    
    // Предлагаем удалить зависимости
    if !deps_to_remove.is_empty() {
        println!("\n{}", format!("{} dependencies are no longer needed:", deps_to_remove.len()).yellow());
        for (_, name) in &deps_to_remove {
            println!("  - {}", name);
        }
        print!("\n{}", "Remove them too? [y/N] ".bold());
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() == "y" {
            for (path, name) in deps_to_remove {
                println!("  Removing {}...", name);
                let _ = fs::remove_dir_all(&path);
            }
        }
    }
    
    // Спрашиваем подтверждение
    print!("\n{}", "Remove this package? [y/N] ".bold());
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" {
        println!("{}", "Removal cancelled.".yellow());
        return;
    }
    
    // Удаляем симлинки
    println!("\n{}", "Removing symlinks...".bold());
    let bin_dir = format!("{}/.local/bin", home);
    if Path::new(&bin_dir).exists() {
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Ok(target) = fs::read_link(&path) {
                        if target.starts_with(&pkg_path) {
                            println!("  Removing: {}", path.display());
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }
    
    // Удаляем директорию пакета
    println!("{}", "Removing package files...".bold());
    match fs::remove_dir_all(&pkg_path) {
        Ok(_) => {
            println!("  ✓ Removed: {}", pkg_path.display());
        }
        Err(e) => {
            println!("  ✗ Failed to remove: {}", e);
            return;
        }
    }
    
    // Проверяем, опустел ли репозиторий
    if let Some(parent) = pkg_path.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            if entries.count() == 0 {
                println!("  → Repository directory is empty, removing...");
                let _ = fs::remove_dir(parent);
            }
        }
    }
    
    println!("\n{}", format!("✓ {} removed successfully!", pkg_full_name).green().bold());
}
