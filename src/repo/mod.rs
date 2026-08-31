use crate::config::{Config, RepoConfig};
use colored::*;
use std::collections::HashMap;
use std::io::Write;

pub mod arch;
pub mod deb;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub repo: String,
    pub size: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<String>,
}

impl Package {
    pub fn display(&self, index: usize) {
        let size_str = self.size.as_deref().unwrap_or("???");
        let license_str = self.license.as_deref().unwrap_or("Unknown");
        
        println!(
            "({}) {}-{} ({}) [{}] [{}]",
            index + 1,
            self.name.cyan(),
            self.version.green(),
            self.repo.yellow(),
            size_str,
            license_str
        );
    }
}

static mut REPO_CACHE: Option<HashMap<String, Vec<Package>>> = None;

fn get_cache() -> &'static mut HashMap<String, Vec<Package>> {
    unsafe {
        if REPO_CACHE.is_none() {
            REPO_CACHE = Some(HashMap::new());
        }
        REPO_CACHE.as_mut().unwrap()
    }
}

pub fn search_packages(pattern: &str) {
    let config = Config::load();
    
    // Проверяем кеш
    let cache = get_cache();
    let has_cache = !cache.is_empty();
    
    if has_cache {
        // Если кеш есть — используем его без вопросов
        println!("\n{}", "Using cached package lists...".dimmed());
    } else {
        // Если кеша нет — загружаем все репозитории
        println!("\n{}", "Loading package lists...".bold());
        for repo in config.repos {
            if repo.enabled != Some(true) {
                continue;
            }
            load_repo_packages(&repo);
        }
    }
    
    let mut all_packages = Vec::new();
    for (_, packages) in cache.iter() {
        all_packages.extend(packages.clone());
    }
    
    let results: Vec<&Package> = all_packages
        .iter()
        .filter(|p| p.name.contains(pattern))
        .collect();
    
    if results.is_empty() {
        println!("\n{}", format!("No packages found for '{}'", pattern).red());
        return;
    }
    
    println!("\n{}", format!("Found {} results:", results.len()).green().bold());
    
    for (i, pkg) in results.iter().enumerate() {
        pkg.display(i);
    }
}

pub fn find_all_packages(pkgname: &str) -> Vec<Package> {
    let config = Config::load();
    let mut found = Vec::new();
    
    let cache = get_cache();
    let has_cache = !cache.is_empty();
    
    if !has_cache {
        println!("\n{}", "Loading package lists...".bold());
        for repo in config.repos {
            if repo.enabled != Some(true) {
                continue;
            }
            load_repo_packages(&repo);
        }
    }
    
    for (_, packages) in cache.iter() {
        for pkg in packages {
            if pkg.name == pkgname || pkg.name.contains(pkgname) {
                found.push(pkg.clone());
            }
        }
    }
    
    found
}

pub fn find_package_exact(pkgname: &str) -> Option<Package> {
    let config = Config::load();
    let cache = get_cache();
    
    // Если кеша нет — загружаем
    if cache.is_empty() {
        for repo in config.repos {
            if repo.enabled != Some(true) {
                continue;
            }
            load_repo_packages(&repo);
        }
    }
    
    for (_, packages) in cache.iter() {
        for pkg in packages {
            if pkg.name == pkgname {
                return Some(pkg.clone());
            }
        }
    }
    
    None
}

pub fn find_package_in_repo(pkgname: &str, repo_name: &str) -> Option<Package> {
    let config = Config::load();
    let cache = get_cache();
    
    if cache.is_empty() {
        for repo in config.repos {
            if repo.enabled != Some(true) {
                continue;
            }
            load_repo_packages(&repo);
        }
    }
    
    for (_, packages) in cache.iter() {
        for pkg in packages {
            if pkg.repo == repo_name && pkg.name == pkgname {
                return Some(pkg.clone());
            }
        }
    }
    
    None
}

fn load_repo_packages(repo: &RepoConfig) -> Vec<Package> {
    let cache_key = format!("{}_{}", repo.name, repo.repo_type);
    
    let cache = get_cache();
    if let Some(packages) = cache.get(&cache_key) {
        return packages.clone();
    }
    
    let packages = match repo.repo_type.as_str() {
        "arch" => {
            match arch::load_arch_repo(repo) {
                Ok(p) => {
                    println!("  ✓ {} loaded {} packages", repo.name, p.len());
                    p
                },
                Err(e) => {
                    println!("  ⚠ Failed to load arch repo {}: {}", repo.name, e);
                    Vec::new()
                }
            }
        }
        "deb" => {
            match deb::load_deb_repo(repo) {
                Ok(p) => {
                    println!("  ✓ {} loaded {} packages", repo.name, p.len());
                    p
                },
                Err(e) => {
                    println!("  ⚠ Failed to load deb repo {}: {}", repo.name, e);
                    Vec::new()
                }
            }
        }
        _ => {
            println!("  ⚠ Unknown repo type: {}", repo.repo_type);
            Vec::new()
        }
    };
    
    cache.insert(cache_key, packages.clone());
    packages
}
