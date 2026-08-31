use crate::config::RepoConfig;
use crate::fetch;
use crate::repo::Package;
use colored::*;
use std::fs;

pub fn load_apk_repo(repo: &RepoConfig) -> Result<Vec<Package>, String> {
    println!("  → {}: loading APK repo...", repo.name.cyan());
    
    // Пробуем разные варианты APKINDEX
    let index_urls = vec![
        format!("{}/APKINDEX.tar.gz", repo.url),
        format!("{}/APKINDEX.gz", repo.url),
    ];
    
    let mut index_path = String::new();
    let mut loaded = false;
    
    for url in index_urls {
        let path = format!("/tmp/{}_APKINDEX.gz", repo.name.replace("/", "_"));
        if let Ok(_) = fetch::download_file_silent(&url, &path) {
            index_path = path;
            loaded = true;
            break;
        }
    }
    
    if !loaded {
        println!("  ⚠ Failed to download APKINDEX");
        return Ok(Vec::new());
    }
    
    // Читаем gzip напрямую
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    let file = match fs::File::open(&index_path) {
        Ok(f) => f,
        Err(e) => {
            println!("  ⚠ Failed to open APKINDEX: {}", e);
            return Ok(Vec::new());
        }
    };
    
    let gz = GzDecoder::new(file);
    let mut reader = std::io::BufReader::new(gz);
    let mut content = String::new();
    if let Err(e) = reader.read_to_string(&mut content) {
        println!("  ⚠ Failed to read APKINDEX: {}", e);
        return Ok(Vec::new());
    }
    
    let mut packages = Vec::new();
    let mut current_pkg = Package {
        name: String::new(),
        version: String::new(),
        repo: repo.name.clone(),
        size: None,
        license: None,
        dependencies: Vec::new(),
    };
    
    for line in content.lines() {
        let line = line.trim();
        
        if line.is_empty() {
            if !current_pkg.name.is_empty() && !current_pkg.version.is_empty() {
                packages.push(current_pkg.clone());
            }
            current_pkg = Package {
                name: String::new(),
                version: String::new(),
                repo: repo.name.clone(),
                size: None,
                license: None,
                dependencies: Vec::new(),
            };
            continue;
        }
        
        if let Some(rest) = line.strip_prefix("P:") {
            current_pkg.name = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("V:") {
            current_pkg.version = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("S:") {
            if let Ok(size) = rest.parse::<u64>() {
                if size > 1024 * 1024 {
                    current_pkg.size = Some(format!("{:.2} MiB", size as f64 / 1024.0 / 1024.0));
                } else if size > 1024 {
                    current_pkg.size = Some(format!("{:.2} KiB", size as f64 / 1024.0));
                } else {
                    current_pkg.size = Some(format!("{} B", size));
                }
            }
        } else if let Some(rest) = line.strip_prefix("L:") {
            current_pkg.license = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("D:") {
            for dep in rest.split(' ') {
                if !dep.is_empty() {
                    current_pkg.dependencies.push(dep.to_string());
                }
            }
        }
    }
    
    if !current_pkg.name.is_empty() && !current_pkg.version.is_empty() {
        packages.push(current_pkg);
    }
    
    let _ = fs::remove_file(&index_path);
    
    println!("  ✓ {} APK packages loaded", packages.len());
    Ok(packages)
}
