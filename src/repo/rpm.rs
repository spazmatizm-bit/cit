use crate::config::RepoConfig;
use crate::fetch;
use crate::repo::Package;
use colored::*;
use std::fs;
use flate2::read::GzDecoder;
use std::io::Read;
use std::process::Command;

pub fn load_rpm_repo(repo: &RepoConfig) -> Result<Vec<Package>, String> {
    println!("  → {}: loading RPM repo...", repo.name.cyan());
    
    let repomd_url = format!("{}/repodata/repomd.xml", repo.url);
    let repomd_path = format!("/tmp/{}_repomd.xml", repo.name.replace("/", "_"));
    
    if let Err(e) = fetch::download_file_silent(&repomd_url, &repomd_path) {
        println!("  ⚠ Failed to download repomd.xml: {}", e);
        return Ok(Vec::new());
    }
    
    let content = match fs::read_to_string(&repomd_path) {
        Ok(c) => c,
        Err(e) => {
            println!("  ⚠ Failed to read repomd.xml: {}", e);
            return Ok(Vec::new());
        }
    };
    
    // Ищем primary.xml.gz ИЛИ primary.sqlite.xz в repomd.xml
    let primary_href = content.lines()
        .find(|line| line.contains("primary.xml.gz") || line.contains("primary.sqlite.xz"))
        .and_then(|line| {
            line.split('"')
                .find(|s| s.contains("primary.xml.gz") || s.contains("primary.sqlite.xz"))
                .map(|s| s.to_string())
        });
    
    let primary_path = match primary_href {
        Some(href) => {
            let primary_url = format!("{}/{}", repo.url, href);
            let path = format!("/tmp/{}_primary.xml", repo.name.replace("/", "_"));
            
            if let Err(e) = fetch::download_file_silent(&primary_url, &path) {
                println!("  ⚠ Failed to download primary: {}", e);
                return Ok(Vec::new());
            }
            path
        }
        None => {
            println!("  ⚠ primary.xml not found in repomd.xml");
            return Ok(Vec::new());
        }
    };
    
    // Определяем формат файла и распаковываем
    let xml_content = if primary_path.ends_with(".xz") {
        // Распаковываем .xz
        let output = Command::new("xz")
            .args(&["-d", "-c", &primary_path])
            .output()
            .map_err(|e| format!("Failed to run xz: {}", e))?;
        
        if !output.status.success() {
            return Err("Failed to decompress xz".to_string());
        }
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // Распаковываем .gz
        let file = match fs::File::open(&primary_path) {
            Ok(f) => f,
            Err(e) => {
                println!("  ⚠ Failed to open primary file: {}", e);
                return Ok(Vec::new());
            }
        };
        
        let gz = GzDecoder::new(file);
        let mut reader = std::io::BufReader::new(gz);
        let mut content = String::new();
        if let Err(e) = reader.read_to_string(&mut content) {
            println!("  ⚠ Failed to read primary.xml: {}", e);
            return Ok(Vec::new());
        }
        content
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
    let mut in_package = false;
    
    for line in xml_content.lines() {
        if line.contains("<package ") {
            in_package = true;
            current_pkg = Package {
                name: String::new(),
                version: String::new(),
                repo: repo.name.clone(),
                size: None,
                license: None,
                dependencies: Vec::new(),
            };
        } else if in_package {
            if let Some(start) = line.find("<name>") {
                let start_idx = start + 6;
                if let Some(end) = line[start_idx..].find("</name>") {
                    current_pkg.name = line[start_idx..start_idx + end].to_string();
                }
            }
            if let Some(start) = line.find("ver=\"") {
                let start_idx = start + 5;
                if let Some(end) = line[start_idx..].find('"') {
                    current_pkg.version = line[start_idx..start_idx + end].to_string();
                }
            }
            if let Some(start) = line.find("package=\"") {
                let start_idx = start + 9;
                if let Some(end) = line[start_idx..].find('"') {
                    if let Ok(size) = line[start_idx..start_idx + end].parse::<u64>() {
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
            if line.contains("</package>") {
                if !current_pkg.name.is_empty() && !current_pkg.version.is_empty() {
                    packages.push(current_pkg.clone());
                }
                in_package = false;
            }
        }
    }
    
    let _ = fs::remove_file(&repomd_path);
    let _ = fs::remove_file(&primary_path);
    
    println!("  ✓ {} RPM packages loaded", packages.len());
    Ok(packages)
}
