use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub url: String,
    #[serde(rename = "type")]
    pub repo_type: String,
    pub enabled: Option<bool>,
    pub suite: Option<String>,      // для Debian: stable, bookworm, etc
    pub distro: Option<String>,    // arch, debian, fedora
    pub version: Option<String>,   // для Fedora: 41, 40, etc
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub repos: Vec<RepoConfig>,
}

impl Config {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_path = format!("{}/.cit.conf", home);
        
        if !Path::new(&config_path).exists() {
            eprintln!("Config file not found at {}, using defaults", config_path);
            return Self::defaults();
        }
        
        let content = fs::read_to_string(&config_path).unwrap_or_default();
        
        let mut repos = Vec::new();
        let mut current_repo: Option<RepoConfig> = None;
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('[') && line.ends_with(']') {
                if let Some(repo) = current_repo.take() {
                    repos.push(repo);
                }
                
                let name = line[1..line.len()-1].to_string();
                current_repo = Some(RepoConfig {
                    name,
                    url: String::new(),
                    repo_type: String::new(),
                    enabled: Some(true),
                    suite: None,
                    distro: None,
                    version: None,
                });
                continue;
            }
            
            if let Some(ref mut repo) = current_repo {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    
                    match key {
                        "url" => repo.url = value.to_string(),
                        "type" => repo.repo_type = value.to_string(),
                        "enabled" => repo.enabled = Some(value == "1" || value == "true"),
                        "suite" => repo.suite = Some(value.to_string()),
                        "distro" => repo.distro = Some(value.to_string()),
                        "version" => repo.version = Some(value.to_string()),
                        _ => {}
                    }
                }
            }
        }
        
        if let Some(repo) = current_repo {
            repos.push(repo);
        }
        
        Self { repos }
    }
    
    fn defaults() -> Self {
        Self {
            repos: vec![
                RepoConfig {
                    name: "arch/core".to_string(),
                    url: "https://mirror.yandex.ru/archlinux/core/os/x86_64".to_string(),
                    repo_type: "arch".to_string(),
                    enabled: Some(true),
                    suite: None,
                    distro: Some("arch".to_string()),
                    version: None,
                },
                RepoConfig {
                    name: "arch/extra".to_string(),
                    url: "https://mirror.yandex.ru/archlinux/extra/os/x86_64".to_string(),
                    repo_type: "arch".to_string(),
                    enabled: Some(true),
                    suite: None,
                    distro: Some("arch".to_string()),
                    version: None,
                },
                RepoConfig {
                    name: "debian/stable".to_string(),
                    url: "http://mirror.yandex.ru/debian".to_string(),
                    repo_type: "deb".to_string(),
                    enabled: Some(true),
                    suite: Some("stable".to_string()),
                    distro: Some("debian".to_string()),
                    version: None,
                },
            ],
        }
    }
}
