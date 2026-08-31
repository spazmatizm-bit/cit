use crate::config::Config;
use std::fs;
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader};

pub struct PackageFinder;

impl PackageFinder {
    pub fn find_deb_url(pkgname: &str, version: &str, repo_name: &str) -> Result<String, String> {
        let config = Config::load();
        
        for repo in config.repos {
            if repo.name != repo_name || repo.repo_type != "deb" {
                continue;
            }
            
            let packages_url = format!("{}/dists/stable/main/binary-amd64/Packages.gz", repo.url);
            let packages_path = format!("/tmp/{}_Packages_find.gz", repo.name);
            
            // Скачиваем Packages.gz
            let client = reqwest::blocking::Client::new();
            let response = client.get(&packages_url)
                .send()
                .map_err(|e| e.to_string())?;
            
            if !response.status().is_success() {
                continue;
            }
            
            let bytes = response.bytes().map_err(|e| e.to_string())?;
            fs::write(&packages_path, bytes).map_err(|e| e.to_string())?;
            
            let file = fs::File::open(&packages_path).map_err(|e| e.to_string())?;
            let gz = GzDecoder::new(file);
            let reader = BufReader::new(gz);
            
            let mut current_pkg = String::new();
            let mut current_version = String::new();
            let mut current_filename = String::new();
            let mut in_pkg = false;
            
            for line in reader.lines() {
                let line = line.map_err(|e| e.to_string())?;
                
                if line.is_empty() {
                    if in_pkg && current_pkg == pkgname && current_version == version {
                        let _ = fs::remove_file(&packages_path);
                        return Ok(format!("{}/{}", repo.url, current_filename));
                    }
                    current_pkg.clear();
                    current_version.clear();
                    current_filename.clear();
                    in_pkg = false;
                    continue;
                }
                
                if line.starts_with("Package: ") {
                    current_pkg = line[9..].to_string();
                    in_pkg = true;
                } else if line.starts_with("Version: ") {
                    current_version = line[9..].to_string();
                } else if line.starts_with("Filename: ") {
                    current_filename = line[10..].to_string();
                }
            }
            
            let _ = fs::remove_file(&packages_path);
        }
        
        Err("Package not found in repositories".to_string())
    }
}
