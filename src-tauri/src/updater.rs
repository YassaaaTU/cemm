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
    
    log::debug!("Checking for updates...");
    log::debug!("Current version: {}", current_version);
    log::debug!("Repository: {}", repo);
    
    // Fetch latest release from GitHub
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    
    log::debug!("Fetching from URL: {}", url);
    
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

    log::debug!("Latest release tag: {}", release.tag_name);
    log::debug!("Is prerelease: {}", release.prerelease);
    log::debug!("Assets count: {}", release.assets.len());

    // Skip prerelease versions
    if release.prerelease {
        log::debug!("Skipping prerelease version");
        return Ok(UpdateInfo {
            available: false,
            current_version: current_version.to_string(),
            latest_version: release.tag_name.trim_start_matches('v').to_string(),
            download_url: None,
            asset_name: None,
            size: None,        });
    }

    // Compare versions - handle both "v1.2.0" and "cemm-v1.2.0" formats
    let latest_version = release.tag_name
        .trim_start_matches("cemm-v")  // Remove "cemm-v" prefix first
        .trim_start_matches('v');     // Then remove standalone "v" prefix
    log::debug!("Comparing versions - Current: '{}', Latest: '{}'", current_version, latest_version);
    
    let update_available = is_newer_version(current_version, latest_version)?;
    log::debug!("Update available: {}", update_available);

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
    log::debug!("Starting download - URL: {}, Asset: {}", download_url, asset_name);
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&asset_name);
    
    log::debug!("Download destination: {}", file_path.display());

    // Download file
    let client = reqwest::Client::new();
    log::debug!("Making HTTP request to: {}", download_url);
    
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("Download failed: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

    if !response.status().is_success() {
        let error_msg = format!("Download failed with status: {}", response.status());
        log::error!("{}", error_msg);
        return Err(error_msg);
    }
    
    log::debug!("HTTP request successful, reading bytes...");

    let bytes = response
        .bytes()
        .await
        .map_err(|e| {
            let error_msg = format!("Failed to read download: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

    log::debug!("Downloaded {} bytes, writing to file...", bytes.len());

    // Write to temp file
    std::fs::write(&file_path, bytes)
        .map_err(|e| {
            let error_msg = format!("Failed to write file: {}", e);
            log::error!("{}", error_msg);
            error_msg
        })?;

    log::debug!("File written successfully: {}", file_path.display());
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn install_updater_file(file_path: String) -> Result<(), String> {
    let path = PathBuf::from(&file_path);
    
    log::debug!("Starting installation - File: {}", file_path);
    
    if !path.exists() {
        let error_msg = "Update file not found".to_string();
        log::error!("{}", error_msg);
        return Err(error_msg);
    }

    log::debug!("File exists, OS: {}", std::env::consts::OS);

    // Start the relaunch helper BEFORE running the installer
    if let Err(e) = spawn_relaunch_helper() {
        log::warn!("Failed to spawn relaunch helper: {}", e);
    }

    let result = match std::env::consts::OS {
        "windows" => install_windows_update(&path),
        "macos" => install_macos_update(&path),
        "linux" => install_linux_update(&path),
        _ => {
            let error_msg = "Unsupported platform for auto-update".to_string();
            log::error!("{}", error_msg);
            return Err(error_msg);
        }
    };

    match result {
        Ok(_) => {
            log::debug!("Installation started successfully");
            // Clean up temp file
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("Failed to clean up temp file: {}", e);
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Installation failed: {}", e);
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
                log::debug!("Selected asset for Windows: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".msi") {
                log::debug!("Selected asset for Windows: {}", asset.name);
                return Ok(asset);
            }
        }
    }
    // macOS: prefer .dmg, then .zip
    else if target_os == "macos" {
        for asset in assets {
            if asset.name.ends_with(".dmg") {
                log::debug!("Selected asset for macOS: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".zip") {
                log::debug!("Selected asset for macOS: {}", asset.name);
                return Ok(asset);
            }
        }
    }
    // Linux: prefer .AppImage, then .deb, then .tar.gz
    else if target_os == "linux" {
        for asset in assets {
            if asset.name.ends_with(".AppImage") {
                log::debug!("Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".deb") {
                log::debug!("Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
        for asset in assets {
            if asset.name.ends_with(".tar.gz") {
                log::debug!("Selected asset for Linux: {}", asset.name);
                return Ok(asset);
            }
        }
    }

    // Fallback: just pick the first asset and log a warning
    if let Some(asset) = assets.first() {
        log::warn!("No platform-specific asset found, using first asset: {}", asset.name);
        return Ok(asset);
    }

    Err("No suitable asset found for this platform".to_string())
}

fn is_newer_version(current: &str, latest: &str) -> Result<bool, String> {
    log::debug!("Parsing versions - Current: '{}', Latest: '{}'", current, latest);
    
    // Parse semantic versions
    let current_parts: Vec<u32> = current
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    
    let latest_parts: Vec<u32> = latest
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    log::debug!("Current parts: {:?}", current_parts);
    log::debug!("Latest parts: {:?}", latest_parts);

    // Compare major.minor.patch
    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        log::debug!("Comparing part {}: {} vs {}", i, current_part, latest_part);
        
        if latest_part > current_part {
            log::debug!("Latest is newer at part {}", i);
            return Ok(true);
        } else if latest_part < current_part {
            log::debug!("Current is newer at part {}", i);
            return Ok(false);
        }
    }
    
    log::debug!("Versions are equal");
    Ok(false) // Versions are equal
}

fn install_windows_update(path: &PathBuf) -> Result<(), String> {
    log::debug!("Installing Windows update: {}", path.display());
    
    // For .msi files
    if path.extension().and_then(|s| s.to_str()) == Some("msi") {
        log::debug!("Detected MSI installer");
        
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
                log::debug!("MSI installer started with UAC elevation");
                return Ok(());
            },
            Err(e) => {
                log::warn!("UAC elevation failed: {}, trying fallback", e);
                // Fallback to normal execution
                match Command::new("msiexec")
                    .args(&["/i", &path.to_string_lossy(), "/quiet", "/norestart"])
                    .spawn() {
                    Ok(_child) => {
                        log::debug!("MSI installer started without elevation");
                        return Ok(());
                    },
                    Err(e) => return Err(format!("Failed to start MSI installer: {}", e))
                }
            }
        }
    }
    // For .exe files  
    else if path.extension().and_then(|s| s.to_str()) == Some("exe") {
        log::debug!("Detected EXE installer");
        // Run the installer
        match Command::new(&path)
            .args(&["/S"]) // Silent install flag
            .spawn() {
            Ok(_child) => {
                log::debug!("EXE installer started");
                return Ok(());
            },
            Err(e) => return Err(format!("Failed to start EXE installer: {}", e))
        }
    } else {
        let error_msg = format!("Unsupported installer format: {:?}", path.extension());
        log::error!("{}", error_msg);
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

// Cross-platform helper to spawn a detached process that will relaunch the app after update
fn spawn_relaunch_helper() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {}", e))?;
    
    let exe_path = current_exe.to_string_lossy().to_string();
    log::debug!("Spawning relaunch helper for: {}", exe_path);

    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to create a detached process that waits and then launches the app
        let script = format!(
            "Start-Process powershell -ArgumentList '-Command', 'Start-Sleep -Seconds 8; Start-Process \\\"{}\\\"; exit' -WindowStyle Hidden",
            exe_path
        );
        
        Command::new("powershell")
            .args(&["-Command", &script])
            .spawn()
            .map_err(|e| format!("Failed to spawn Windows relaunch helper: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        // Use shell script to wait and relaunch
        let script = format!("sleep 8 && open \"{}\" &", exe_path);
        
        Command::new("sh")
            .args(&["-c", &script])
            .spawn()
            .map_err(|e| format!("Failed to spawn macOS relaunch helper: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        // Use shell script to wait and relaunch  
        let script = format!("sleep 8 && \"{}\" &", exe_path);
        
        Command::new("sh")
            .args(&["-c", &script])
            .spawn()
            .map_err(|e| format!("Failed to spawn Linux relaunch helper: {}", e))?;
    }

    log::debug!("Relaunch helper spawned successfully");
    Ok(())
}
