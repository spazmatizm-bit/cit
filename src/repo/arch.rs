use crate::config::RepoConfig;
use crate::fetch;
use crate::repo::Package;
use std::fs;
use flate2::read::GzDecoder;
use tar::Archive;

pub fn load_arch_repo(repo: &RepoConfig) -> Result<Vec<Package>, String> {
    let repo_name = repo.name.split('/').last().unwrap_or(&repo.name);
    let db_name = match repo_name {
        "core" => "core.db",
        "extra" => "extra.db",
        "multilib" => "multilib.db",
        _ => &format!("{}.db", repo_name),
    };
    
    let db_url = format!("{}/{}", repo.url, db_name);
    let db_path = format!("/tmp/{}.db", repo.name.replace("/", "_"));
    
    match fetch::download_file_silent(&db_url, &db_path) {
        Ok(_) => {},
        Err(e) => {
            println!("  ⚠ Failed to download {}: {}", db_name, e);
            return Ok(Vec::new());
        }
    }
    
    let file = match fs::File::open(&db_path) {
        Ok(f) => f,
        Err(e) => {
            println!("  ⚠ Failed to open {}: {}", db_name, e);
            return Ok(Vec::new());
        }
    };
    
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);
    
    let extract_dir = format!("/tmp/{}_extract", repo.name.replace("/", "_"));
    let _ = fs::remove_dir_all(&extract_dir);
    
    if let Err(e) = archive.unpack(&extract_dir) {
        println!("  ⚠ Failed to unpack {}: {}", db_name, e);
        let _ = fs::remove_file(&db_path);
        return Ok(Vec::new());
    }
    
    let mut packages = Vec::new();
    let entries = match fs::read_dir(&extract_dir) {
        Ok(e) => e,
        Err(e) => {
            println!("  ⚠ Failed to read extracted dir: {}", e);
            let _ = fs::remove_file(&db_path);
            let _ = fs::remove_dir_all(&extract_dir);
            return Ok(Vec::new());
        }
    };
    
    let mut count = 0;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        
        if path.is_dir() {
            let pkg_name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            let desc_path = path.join("desc");
            
            if desc_path.exists() {
                let content = match fs::read_to_string(&desc_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                
                let mut version = String::new();
                let mut pkg_size = None;
                let mut license = None;
                let mut dependencies = Vec::new();
                
                let mut lines = content.lines().peekable();
                while let Some(line) = lines.next() {
                    if line == "%VERSION%" {
                        if let Some(ver) = lines.next() {
                            version = ver.to_string();
                        }
                    } else if line == "%CSIZE%" {
                        if let Some(size) = lines.next() {
                            if let Ok(size_num) = size.parse::<u64>() {
                                if size_num > 1024 * 1024 {
                                    pkg_size = Some(format!("{:.2} MiB", size_num as f64 / 1024.0 / 1024.0));
                                } else if size_num > 1024 {
                                    pkg_size = Some(format!("{:.2} KiB", size_num as f64 / 1024.0));
                                } else {
                                    pkg_size = Some(format!("{} B", size_num));
                                }
                            }
                        }
                    } else if line == "%LICENSE%" {
                        if let Some(lic) = lines.next() {
                            license = Some(lic.to_string());
                        }
                    } else if line == "%DEPENDS%" {
                        while let Some(dep) = lines.next() {
                            if dep.starts_with('%') { break; }
                            if !dep.is_empty() {
                                let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                                dependencies.push(dep_name.to_string());
                            }
                        }
                    }
                }
                
                if !pkg_name.is_empty() && !version.is_empty() {
                    // Для Arch: pkg_name уже содержит версию, но мы сохраняем только имя
                    // Например: "fastfetch-2.67.1-1" -> name: "fastfetch", version: "2.67.1-1"
                    let name_parts: Vec<&str> = pkg_name.rsplitn(2, '-').collect();
                    let (pkg_name_clean, pkg_version) = if name_parts.len() == 2 {
                        (name_parts[1].to_string(), name_parts[0].to_string())
                    } else {
                        (pkg_name.clone(), version.clone())
                    };
                    
                    packages.push(Package {
                        name: pkg_name_clean,
                        version: pkg_version,
                        repo: repo.name.clone(),
                        size: pkg_size,
                        license,
                        dependencies,
                    });
                    count += 1;
                }
            }
        }
    }
    
    let _ = fs::remove_file(&db_path);
    let _ = fs::remove_dir_all(&extract_dir);
    
    Ok(packages)
}
