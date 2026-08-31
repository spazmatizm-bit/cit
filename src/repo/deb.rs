use crate::config::RepoConfig;
use crate::fetch;
use crate::repo::Package;
use std::fs;
use std::io::{BufRead, BufReader};
use flate2::read::GzDecoder;

pub fn load_deb_repo(repo: &RepoConfig) -> Result<Vec<Package>, String> {
    let suite = repo.suite.as_deref().unwrap_or("stable");
    let packages_url = format!("{}/dists/{}/main/binary-amd64/Packages.gz", repo.url, suite);
    let packages_path = format!("/tmp/{}_Packages.gz", repo.name.replace("/", "_"));
    
    fetch::download_file_silent(&packages_url, &packages_path)?;
    
    if !std::path::Path::new(&packages_path).exists() {
        return Err("Packages.gz not found".to_string());
    }
    
    let file = fs::File::open(&packages_path).map_err(|e| e.to_string())?;
    let gz = GzDecoder::new(file);
    let reader = BufReader::new(gz);
    
    let mut packages = Vec::new();
    let mut current = Package {
        name: String::new(),
        version: String::new(),
        repo: repo.name.clone(),
        size: None,
        license: None,
        dependencies: Vec::new(),
    };
    let mut in_pkg = false;
    
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        
        if line.is_empty() {
            if in_pkg && !current.name.is_empty() && !current.version.is_empty() {
                packages.push(current.clone());
            }
            current = Package {
                name: String::new(),
                version: String::new(),
                repo: repo.name.clone(),
                size: None,
                license: None,
                dependencies: Vec::new(),
            };
            in_pkg = false;
            continue;
        }
        
        if line.starts_with("Package: ") {
            current.name = line[9..].to_string();
            in_pkg = true;
        } else if line.starts_with("Version: ") {
            current.version = line[9..].to_string();
        } else if line.starts_with("Depends: ") {
            let deps = line[9..].split(',');
            for dep in deps {
                let dep = dep.trim();
                let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                if !dep_name.is_empty() {
                    current.dependencies.push(dep_name.to_string());
                }
            }
        } else if line.starts_with("Size: ") {
            if let Ok(size) = line[6..].parse::<u64>() {
                if size > 1024 * 1024 {
                    current.size = Some(format!("{:.2} MiB", size as f64 / 1024.0 / 1024.0));
                } else if size > 1024 {
                    current.size = Some(format!("{:.2} KiB", size as f64 / 1024.0));
                } else {
                    current.size = Some(format!("{} B", size));
                }
            }
        }
    }
    
    let _ = fs::remove_file(&packages_path);
    Ok(packages)
}
