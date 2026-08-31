use crate::config::RepoConfig;
use crate::fetch;
use crate::repo::Package;
use colored::*;
use std::fs;

pub fn load_xbps_repo(repo: &RepoConfig) -> Result<Vec<Package>, String> {
    println!("  → {}: loading XBPS repo...", repo.name.cyan());
    
    let index_url = format!("{}/index.plist", repo.url);
    let index_path = format!("/tmp/{}_index.plist", repo.name.replace("/", "_"));
    
    if let Err(e) = fetch::download_file_silent(&index_url, &index_path) {
        println!("  ⚠ Failed to download index.plist: {}", e);
        return Ok(Vec::new());
    }
    
    let content = match fs::read_to_string(&index_path) {
        Ok(c) => c,
        Err(e) => {
            println!("  ⚠ Failed to read index.plist: {}", e);
            return Ok(Vec::new());
        }
    };
    
    let mut packages = Vec::new();
    let mut current_pkg = Package {
        name: String::new(),
        version: String::new(),
        repo: repo.name.clone(),
        size: None,
        license: None,
        dependencies: Vec::new(),
    };
    let mut in_pkg = false;
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with("{") {
            in_pkg = true;
            current_pkg = Package {
                name: String::new(),
                version: String::new(),
                repo: repo.name.clone(),
                size: None,
                license: None,
                dependencies: Vec::new(),
            };
        } else if in_pkg && line.starts_with("}") {
            if !current_pkg.name.is_empty() && !current_pkg.version.is_empty() {
                packages.push(current_pkg.clone());
            }
            in_pkg = false;
        } else if in_pkg {
            if let Some(name) = line.split(':').nth(0) {
                let name = name.trim().trim_matches('"');
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim().trim_matches('"').trim_matches(',');
                    match name {
                        "pkgname" => current_pkg.name = value.to_string(),
                        "version" => current_pkg.version = value.to_string(),
                        "filename" => {
                            if let Some(size_str) = value.split('.').nth(0) {
                                if let Ok(size) = size_str.parse::<u64>() {
                                    if size > 1024 * 1024 {
                                        current_pkg.size = Some(format!("{:.2} MiB", size as f64 / 1024.0 / 1024.0));
                                    } else if size > 1024 {
                                        current_pkg.size = Some(format!("{:.2} KiB", size as f64 / 1024.0));
                                    } else {
                                        current_pkg.size = Some(format!("{} B", size));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    let _ = fs::remove_file(&index_path);
    println!("  ✓ {} XBPS packages loaded", packages.len());
    Ok(packages)
}
