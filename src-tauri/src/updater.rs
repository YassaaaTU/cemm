use serde::{Deserialize, Serialize};
use reqwest;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    published_at: String,
    assets: Vec<GitHubAsset>,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub size: Option<u64>,
}

#[tauri::command]
pub async fn check_for_updates(repo: String) -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION"); // Gets version from Cargo.toml
    
    println!("DEBUG: Checking for updates...");
    println!("DEBUG: Current version: {}", current_version);
    println!("DEBUG: Repository: {}", repo);
    
    // Fetch latest release from GitHub
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    
    println!("DEBUG: Fetching from URL: {}", url);
    
    let response = client
        .get(&url)
        .header("User-Agent", "CEMM-App")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release data: {}", e))?;

    println!("DEBUG: Latest release tag: {}", release.tag_name);
    println!("DEBUG: Is prerelease: {}", release.prerelease);
    println!("DEBUG: Assets count: {}", release.assets.len());

    // Skip prerelease versions
    if release.prerelease {
        println!("DEBUG: Skipping prerelease version");
        return Ok(UpdateInfo {
            available: false,
            current_version: current_version.to_string(),
            latest_version: release.tag_name.trim_start_matches('v').to_string(),
            download_url: None,
            asset_name: None,
            size: None,        });
    }

    // Compare versions - handle both "v1.2.0" and "app-v1.2.0" formats
    let latest_version = release.tag_name
        .trim_start_matches("app-v")  // Remove "app-v" prefix first
        .trim_start_matches('v');     // Then remove standalone "v" prefix
    println!("DEBUG: Comparing versions - Current: '{}', Latest: '{}'", current_version, latest_version);
    
    let update_available = is_newer_version(current_version, latest_version)?;
    println!("DEBUG: Update available: {}", update_available);

    if !update_available {
        return Ok(UpdateInfo {
            available: false,
            current_version: current_version.to_string(),
            latest_version: latest_version.to_string(),
            download_url: None,
            asset_name: None,
            size: None,
        });
    }

    // Find appropriate asset for current platform
    let platform_asset = find_platform_asset(&release.assets)?;
    Ok(UpdateInfo {
        available: true,
        current_version: current_version.to_string(),
        latest_version: latest_version.to_string(),
        download_url: Some(platform_asset.browser_download_url.clone()),
        asset_name: Some(platform_asset.name.clone()),
        size: Some(platform_asset.size),
    })
}

#[tauri::command]
pub async fn download_updater_file(download_url: String, asset_name: String) -> Result<String, String> {
    println!("DEBUG: Starting download - URL: {}, Asset: {}", download_url, asset_name);    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&asset_name);
    
    println!("DEBUG: Download destination: {}", file_path.display());

    // Download file
    let client = reqwest::Client::new();
    println!("DEBUG: Making HTTP request to: {}", download_url);
    
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Download failed: {}", e);
            println!("ERROR: {}", error_msg);
            error_msg
        })?;

    if !response.status().is_success() {
        let error_msg = format!("Download failed with status: {}", response.status());
        println!("ERROR: {}", error_msg);
        return Err(error_msg);
    }
    
    println!("DEBUG: HTTP request successful, reading bytes...");

    let bytes = response
        .bytes()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to read download: {}", e);
            println!("ERROR: {}", error_msg);
            error_msg
        })?;

    println!("DEBUG: Downloaded {} bytes, writing to file...", bytes.len());

    // Write to temp file
    std::fs::write(&file_path, bytes)
        .map_err(|e| {
            let error_msg = format!("Failed to write file: {}", e);
            println!("ERROR: {}", error_msg);
            error_msg
        })?;

    println!("DEBUG: File written successfully: {}", file_path.display());
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn install_updater_file(file_path: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);
    
    println!("DEBUG: Starting installation - File: {}", file_path);
    
    if !path.exists() {
        let error_msg = "Update file not found".to_string();
        println!("ERROR: {}", error_msg);
        return Err(error_msg);
    }

    println!("DEBUG: File exists, OS: {}", std::env::consts::OS);

    let result = match std::env::consts::OS {
        "windows" => install_windows_update(&path),
        "macos" => install_macos_update(&path),
        "linux" => install_linux_update(&path),
        _ => {
            let error_msg = "Unsupported platform for auto-update".to_string();
            println!("ERROR: {}", error_msg);
            return Err(error_msg);
        }
    };

    match result {
        Ok(_) => {
            println!("DEBUG: Installation started successfully");
            // Clean up temp file
            if let Err(e) = std::fs::remove_file(&path) {
                println!("WARNING: Failed to clean up temp file: {}", e);
            }
            Ok(())
        }
        Err(e) => {
            println!("ERROR: Installation failed: {}", e);
            Err(e)
        }
    }
}

fn find_platform_asset(assets: &[GitHubAsset]) -> Result<&GitHubAsset, String> {
    let target_os = std::env::consts::OS;
    let _target_arch = std::env::consts::ARCH;

    // Windows: prefer .exe, then .msi
    if target_os == "windows" {
        for asset in assets {
            if asset.name.ends_with(".exe") {
                println!("DEBUG: Selected asset for Windows: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".msi") {
                println!("DEBUG: Selected asset for Windows: {}", asset.name);
                return Ok(asset);
            }
        }
    }
    // macOS: prefer .dmg, then .zip
    else if target_os == "macos" {
        for asset in assets {
            if asset.name.ends_with(".dmg") {
                println!("DEBUG: Selected asset for macOS: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".zip") {
                println!("DEBUG: Selected asset for macOS: {}", asset.name);
                return Ok(asset);
            }
        }
    }
    // Linux: prefer .AppImage, then .deb, then .tar.gz
    else if target_os == "linux" {
        for asset in assets {
            if asset.name.ends_with(".AppImage") {
                println!("DEBUG: Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".deb") {
                println!("DEBUG: Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".tar.gz") {
                println!("DEBUG: Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
    }

    // Fallback: just pick the first asset and log a warning
    if let Some(asset) = assets.first() {
        println!("WARNING: No platform-specific asset found, using first asset: {}", asset.name);
        return Ok(asset);
    }

    Err("No suitable asset found for this platform".to_string())
}

fn is_newer_version(current: &str, latest: &str) -> Result<bool, String> {
    println!("DEBUG: Parsing versions - Current: '{}', Latest: '{}'", current, latest);
    
    // Parse semantic versions
    let current_parts: Vec<u32> = current
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    
    let latest_parts: Vec<u32> = latest
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    println!("DEBUG: Current parts: {:?}", current_parts);
    println!("DEBUG: Latest parts: {:?}", latest_parts);

    // Compare major.minor.patch
    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        println!("DEBUG: Comparing part {}: {} vs {}", i, current_part, latest_part);
        
        if latest_part > current_part {
            println!("DEBUG: Latest is newer at part {}", i);
            return Ok(true);
        } else if latest_part < current_part {
            println!("DEBUG: Current is newer at part {}", i);
            return Ok(false);
        }
    }
    
    println!("DEBUG: Versions are equal");
    Ok(false) // Versions are equal
}

fn install_windows_update(path: &PathBuf) -> Result<(), String> {
    println!("DEBUG: Installing Windows update: {}", path.display());
    
    // For .msi files
    if path.extension().and_then(|s| s.to_str()) == Some("msi") {
        println!("DEBUG: Detected MSI installer");
        
        // Try UAC elevation first
        let result = Command::new("powershell")
            .args(&[
                "-Command", 
                &format!("Start-Process msiexec -ArgumentList '/i', '{}', '/quiet', '/norestart' -Verb RunAs", 
                        path.to_string_lossy())
            ])
            .spawn();
        
        match result {
            Ok(_child) => {
                println!("DEBUG: MSI installer started with UAC elevation");
                return Ok(());
            },
            Err(e) => {
                println!("WARNING: UAC elevation failed: {}, trying fallback", e);
                // Fallback to normal execution
                match Command::new("msiexec")
                    .args(&["/i", &path.to_string_lossy(), "/quiet", "/norestart"])
                    .spawn() {
                    Ok(_child) => {
                        println!("DEBUG: MSI installer started without elevation");
                        return Ok(());
                    },
                    Err(e) => return Err(format!("Failed to start MSI installer: {}", e))
                }
            }
        }
    }
    // For .exe files  
    else if path.extension().and_then(|s| s.to_str()) == Some("exe") {
        println!("DEBUG: Detected EXE installer");
        
        match Command::new(&path)
            .args(&["/S"]) // Silent install flag
            .spawn() {
            Ok(_child) => {
                println!("DEBUG: EXE installer started");
                return Ok(());
            },
            Err(e) => return Err(format!("Failed to start EXE installer: {}", e))
        }
    } else {
        let error_msg = format!("Unsupported installer format: {:?}", path.extension());
        println!("ERROR: {}", error_msg);
        return Err(error_msg);
    }
}

fn install_macos_update(path: &PathBuf) -> Result<(), String> {
    // For .dmg files, open for user to install
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open DMG: {}", e))?;
    
    Ok(())
}

fn install_linux_update(path: &PathBuf) -> Result<(), String> {
    // For AppImage, make executable and launch
    if path.extension().and_then(|s| s.to_str()) == Some("AppImage") {
        // Make executable
        Command::new("chmod")
            .args(&["+x", &path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to make executable: {}", e))?;
        
        // Launch new version
        Command::new(&path)
            .spawn()
            .map_err(|e| format!("Failed to launch new version: {}", e))?;
    }
    
    Ok(())
}
