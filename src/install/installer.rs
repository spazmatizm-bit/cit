use crate::config::Config;
use crate::fetch;
use crate::repo::Package;
use crate::init::InitSystem;
use colored::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

pub struct Installer;

impl Installer {
    pub fn new() -> Self {
        Self
    }

    pub fn is_installed(&self, pkg: &Package) -> bool {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let install_dir = format!("{}/.citadel/{}/{}", home, pkg.repo, pkg.name);
        Path::new(&install_dir).exists()
    }

    pub fn create_symlinks_forced(&self, install_dir: &str, pkg: &Package, bin_dir: &str) -> Result<(), String> {
        self.create_symlinks(install_dir, pkg, bin_dir)
    }

    pub fn install_package(&self, pkg: &Package) -> Result<(), String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let install_dir = format!("{}/.citadel/{}/{}", home, pkg.repo, pkg.name);
        
        if Path::new(&install_dir).exists() {
            println!("  ✓ {} already installed", pkg.name.green());
            return Ok(());
        }
        
        let tmp_dir = format!("{}/.cit_cache", home);
        fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
        
        let url = self.get_package_url(pkg)?;
        let pkg_path = format!("{}/{}.pkg", tmp_dir, pkg.name);
        
        let size_str = pkg.size.as_deref().unwrap_or("???");
        println!("  {} [{}] - {} ...", pkg.name.cyan(), pkg.repo.yellow(), size_str);
        fetch::download_file(&url, &pkg_path, &format!("downloading {}", pkg.name))?;
        
        fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create dir: {}", e))?;
        self.extract_package(&pkg_path, &install_dir, pkg)?;
        
        let bin_dir = format!("{}/.local/bin", home);
        fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;
        self.create_symlinks(&install_dir, pkg, &bin_dir)?;
        
        self.update_ld_library_path(&install_dir, &home)?;
        
        // Определяем init и включаем сервисы
        let init = InitSystem::detect();
        let service_dir = init.service_dir();
        let service_path = format!("{}/etc/{}", install_dir, service_dir.trim_start_matches('/'));
        
        if Path::new(&service_path).exists() {
            println!("  → Detected service files, enabling for {:?}...", init);
            let service_name = pkg.name.clone();
            if let Err(e) = init.enable_service(&service_name) {
                println!("  ⚠ Failed to enable service: {}", e);
            } else {
                println!("  ✓ Service enabled for {:?}", init);
            }
        }
        
        let _ = fs::remove_file(&pkg_path);
        println!("  ✓ {}", format!("{} installed", pkg.name).green());
        Ok(())
    }

    fn get_package_url(&self, pkg: &Package) -> Result<String, String> {
        let config = Config::load();
        
        for repo in config.repos {
            if repo.name == pkg.repo {
                return match repo.repo_type.as_str() {
                    "arch" => {
                        let filename = format!("{}-{}-x86_64.pkg.tar.zst", pkg.name, pkg.version);
                        let url = format!("{}/{}", repo.url, filename);
                        
                        let client = reqwest::blocking::Client::new();
                        if let Ok(response) = client.head(&url).send() {
                            if response.status().is_success() {
                                return Ok(url);
                            }
                        }
                        
                        let fallback_filename = format!("{}-{}.pkg.tar.zst", pkg.name, pkg.version);
                        let fallback_url = format!("{}/{}", repo.url, fallback_filename);
                        Ok(fallback_url)
                    }
                    "deb" => {
                        let suite = repo.suite.as_deref().unwrap_or("stable");
                        let packages_url = format!("{}/dists/{}/main/binary-amd64/Packages.gz", repo.url, suite);
                        let packages_path = format!("/tmp/{}_Packages_find.gz", repo.name.replace("/", "_"));
                        
                        let client = reqwest::blocking::Client::new();
                        let response = client.get(&packages_url)
                            .send()
                            .map_err(|e| format!("Failed to fetch packages: {}", e))?;
                        
                        if !response.status().is_success() {
                            return Err("Failed to fetch Packages.gz".to_string());
                        }
                        
                        let bytes = response.bytes().map_err(|e| e.to_string())?;
                        fs::write(&packages_path, bytes).map_err(|e| e.to_string())?;
                        
                        use flate2::read::GzDecoder;
                        use std::io::{BufRead, BufReader};
                        
                        let file = fs::File::open(&packages_path).map_err(|e| e.to_string())?;
                        let gz = GzDecoder::new(file);
                        let reader = BufReader::new(gz);
                        
                        let mut current_pkg = String::new();
                        let mut current_version = String::new();
                        let mut current_filename = String::new();
                        let mut in_pkg = false;
                        let mut found = false;
                        
                        for line in reader.lines() {
                            let line = line.map_err(|e| e.to_string())?;
                            
                            if line.is_empty() {
                                if in_pkg && current_pkg == pkg.name && current_version == pkg.version {
                                    found = true;
                                    break;
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
                        
                        if found && !current_filename.is_empty() {
                            Ok(format!("{}/{}", repo.url, current_filename))
                        } else {
                            let first_letter = pkg.name.chars().next().unwrap_or('a').to_string();
                            let version_clean = pkg.version.replace(":", "");
                            let filename = format!("{}_{}_amd64.deb", pkg.name, version_clean);
                            Ok(format!("{}/pool/main/{}/{}/{}", repo.url, first_letter, pkg.name, filename))
                        }
                    }
                    "xbps" => {
                        // XBPS пакеты: <pkgname>-<version>.<arch>.xbps
                        let filename = format!("{}-{}.x86_64.xbps", pkg.name, pkg.version);
                        Ok(format!("{}/{}", repo.url, filename))
                    }
                    "apk" => {
                        // APK пакеты: <pkgname>-<version>.apk
                        let filename = format!("{}-{}.apk", pkg.name, pkg.version);
                        Ok(format!("{}/{}", repo.url, filename))
                    }
                    _ => Err(format!("Unsupported repo type: {}", repo.repo_type)),
                };
            }
        }
        
        Err(format!("Repository {} not found", pkg.repo))
    }

    fn extract_package(&self, pkg_path: &str, dest: &str, pkg: &Package) -> Result<(), String> {
        let config = Config::load();
        
        for repo in config.repos {
            if repo.name == pkg.repo {
                return match repo.repo_type.as_str() {
                    "arch" => self.extract_arch(pkg_path, dest),
                    "deb" => self.extract_deb(pkg_path, dest),
                    "xbps" => self.extract_xbps(pkg_path, dest),
                    "apk" => self.extract_apk(pkg_path, dest),
                    _ => Err("Unsupported package format".to_string()),
                };
            }
        }
        
        Err("Repository not found".to_string())
    }

    fn extract_arch(&self, pkg_path: &str, dest: &str) -> Result<(), String> {
        let output = Command::new("tar")
            .args(&["-xf", pkg_path, "-C", dest])
            .output()
            .map_err(|e| e.to_string())?;
        
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok(())
    }

    fn extract_deb(&self, pkg_path: &str, dest: &str) -> Result<(), String> {
        let tmp_dir = format!("/tmp/deb_extract_{}", std::process::id());
        fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create tmp dir: {}", e))?;
        
        let output = Command::new("ar")
            .args(&["x", pkg_path])
            .current_dir(&tmp_dir)
            .output()
            .map_err(|e| format!("ar failed: {}", e))?;
        
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        
        let entries = fs::read_dir(&tmp_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            
            if name.starts_with("data.tar.") {
                let output = Command::new("tar")
                    .args(&["-xf", &name, "-C", dest])
                    .current_dir(&tmp_dir)
                    .output()
                    .map_err(|e| format!("tar failed: {}", e))?;
                
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).to_string());
                }
                break;
            }
        }
        
        let _ = fs::remove_dir_all(&tmp_dir);
        Ok(())
    }
    
    fn extract_xbps(&self, pkg_path: &str, dest: &str) -> Result<(), String> {
        // XBPS — это tar.xz
        let output = Command::new("tar")
            .args(&["-xf", pkg_path, "-C", dest])
            .output()
            .map_err(|e| e.to_string())?;
        
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok(())
    }
    
    fn extract_apk(&self, pkg_path: &str, dest: &str) -> Result<(), String> {
        // APK — это tar.gz
        let output = Command::new("tar")
            .args(&["-xzf", pkg_path, "-C", dest])
            .output()
            .map_err(|e| e.to_string())?;
        
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok(())
    }

    fn create_symlinks(&self, install_dir: &str, pkg: &Package, bin_dir: &str) -> Result<(), String> {
        let suffix = match pkg.repo.as_str() {
            "arch/core" | "arch/extra" | "arch/multilib" => "-arch",
            "debian/stable" | "debian/bookworm" => "-deb",
            "devuan/excalibur" => "-devuan",
            "void/current" => "-void",
            "alpine/edge" | "alpine/community" => "-alpine",
            _ => "",
        };
        
        let bin_dirs = ["usr/bin", "usr/local/bin", "bin"];
        let mut found = false;
        
        for bin_dir_name in &bin_dirs {
            let full_bin_dir = format!("{}/{}", install_dir, bin_dir_name);
            if !Path::new(&full_bin_dir).exists() {
                continue;
            }
            
            let entries = fs::read_dir(&full_bin_dir).map_err(|e| format!("Failed to read dir: {}", e))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let path = entry.path();
                
                let is_executable = if let Ok(metadata) = path.metadata() {
                    let permissions = metadata.permissions();
                    permissions.mode() & 0o111 != 0
                } else {
                    false
                };
                
                if is_executable || path.is_symlink() {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let link_path = format!("{}/{}{}", bin_dir, filename, suffix);
                    
                    let _ = fs::remove_file(&link_path);
                    std::os::unix::fs::symlink(&path, &link_path)
                        .map_err(|e| format!("Failed to create symlink: {}", e))?;
                    
                    println!("    ✓ Created: {} -> {}", link_path.cyan(), path.display());
                    found = true;
                }
            }
        }
        
        if !found {
            println!("    ℹ No executables found in {}", install_dir);
        }
        
        Ok(())
    }
    
    fn update_ld_library_path(&self, install_dir: &str, home: &str) -> Result<(), String> {
        let lib_dirs = ["usr/lib", "usr/lib64", "lib", "lib64"];
        let mut found_lib = false;
        
        for lib_dir in &lib_dirs {
            let full_lib_dir = format!("{}/{}", install_dir, lib_dir);
            if Path::new(&full_lib_dir).exists() {
                found_lib = true;
                break;
            }
        }
        
        if found_lib {
            let bashrc_path = format!("{}/.bashrc", home);
            let ld_path_line = format!("\nexport LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"\n", install_dir);
            
            if let Ok(content) = fs::read_to_string(&bashrc_path) {
                if !content.contains(&ld_path_line) {
                    let mut file = fs::OpenOptions::new()
                        .append(true)
                        .open(&bashrc_path)
                        .map_err(|e| format!("Failed to open .bashrc: {}", e))?;
                    use std::io::Write;
                    file.write_all(ld_path_line.as_bytes())
                        .map_err(|e| format!("Failed to write .bashrc: {}", e))?;
                }
            }
            
            let profile_path = format!("{}/.citadel_env.sh", home);
            let profile_content = format!("# Citadel Package Manager Environment\n\
                export LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\"\n\
                export PATH=\"$HOME/.local/bin:$PATH\"\n", install_dir);
            fs::write(&profile_path, profile_content)
                .map_err(|e| format!("Failed to write env script: {}", e))?;
            
            println!("  ℹ Source env: source ~/.citadel_env.sh");
        }
        
        Ok(())
    }
}
