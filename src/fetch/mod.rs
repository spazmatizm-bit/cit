use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::fs;
use std::io::{Read, Write};

pub fn download_file(url: &str, path: &str, _label: &str) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    let total_size = response.content_length().unwrap_or(0);
    
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    
    if total_size > 1024 * 1024 {
        let pb = ProgressBar::new(total_size);
        let style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-");
        pb.set_style(style);
        
        let mut downloaded = 0;
        let mut buffer = [0; 8192];
        let bytes = response.bytes().map_err(|e| e.to_string())?;
        let mut cursor = std::io::Cursor::new(bytes);
        
        loop {
            let n = cursor.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 { break; }
            file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            downloaded += n as u64;
            pb.set_position(downloaded);
        }
        pb.finish_with_message("done");
    } else {
        let bytes = response.bytes().map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

// Тихая загрузка для репозиториев
pub fn download_file_silent(url: &str, path: &str) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }
    
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| e.to_string())?;
    
    Ok(())
}
