// src/browser/driver_updater.rs
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::Deserialize;

// Define Google API response structure
#[derive(Deserialize, Debug)]
struct ChromeVersions {
    milestones: std::collections::HashMap<String, Milestone>,
}

#[derive(Deserialize, Debug)]
struct Milestone {
    downloads: Downloads,
}

#[derive(Deserialize, Debug)]
struct Downloads {
    chromedriver: Option<Vec<Artifact>>,
}

#[derive(Deserialize, Debug)]
struct Artifact {
    platform: String,
    url: String,
}

pub struct ChromeDriver;

impl ChromeDriver {
    /// 1. Main installation function
    /// Check local Chrome version -> Download matching driver -> Return execution path
    pub fn install() -> Result<PathBuf> {
        // Save path: C:\Users\User\.webdrivers
        let target_dir = dirs::home_dir()
            .ok_or(anyhow!("Could not find home directory"))?
            .join(".webdrivers");
            
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)?;
        }
        
        // 2. Check currently installed Chrome version (Windows Registry)
        let current_version = get_local_chrome_version()?;
        let chrome_major: u32 = current_version
            .split('.')
            .next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        println!("Detected Chrome version: {} (Major: {})", current_version, chrome_major);

        // Driver filename (OS specific)
        #[cfg(target_os = "windows")]
        let driver_filename = "chromedriver.exe";
        #[cfg(not(target_os = "windows"))]
        let driver_filename = "chromedriver";
        
        let target_path = target_dir.join(driver_filename);

        // 3. Check and compare existing driver version
        if target_path.exists() {
            if let Some(driver_major) = get_chromedriver_major_version(&target_path) {
                if driver_major == chrome_major {
                    println!("ChromeDriver {} already installed, skipping", driver_major);
                    return Ok(target_path);
                } else {
                    println!("ChromeDriver version mismatch (Driver: {}, Chrome: {}), update required", 
                             driver_major, chrome_major);
                    // Delete existing file
                    let _ = fs::remove_file(&target_path);
                }
            } else {
                println!("Failed to verify existing ChromeDriver version, downloading new one");
                let _ = fs::remove_file(&target_path);
            }
        }

        // 4. Find download URL for the version from Google API
        let url = fetch_driver_url(&chrome_major.to_string())?;
        println!("Download URL found: {}", url);

        // 5. Download and extract
        download_and_extract(&url, &target_dir)?;

        Ok(target_path)
    }
}

/// Run chromedriver.exe --version to parse major version
fn get_chromedriver_major_version(path: &Path) -> Option<u32> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    // Format: "ChromeDriver 143.0.7499.192 ..."
    let version_str = String::from_utf8(output.stdout).ok()?;
    
    // Parse version number after "ChromeDriver "
    let version_part = version_str
        .split_whitespace()
        .nth(1)?;  // "143.0.7499.192"
    
    let major = version_part
        .split('.')
        .next()?
        .parse::<u32>()
        .ok()?;
    
    Some(major)
}

/// Get Chrome version (Cross-platform)
fn get_local_chrome_version() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        let commands = [
            r"(Get-Item 'C:\Program Files\Google\Chrome\Application\chrome.exe').VersionInfo.FileVersion",
            r"(Get-Item 'C:\Program Files (x86)\Google\Chrome\Application\chrome.exe').VersionInfo.FileVersion",
            r"(Get-Item $env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe).VersionInfo.FileVersion",
        ];

        for cmd in commands {
            let output = Command::new("powershell")
                .args(["-Command", cmd])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let version = String::from_utf8(out.stdout)?.trim().to_string();
                    if !version.is_empty() {
                        return Ok(version);
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let paths = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
        ];

        for path in paths {
            if Path::new(path).exists() {
                let output = Command::new(path)
                    .arg("--version")
                    .output();

                if let Ok(out) = output {
                    if out.status.success() {
                        // "Google Chrome 143.0.1234.56" -> "143.0.1234.56"
                        let version = String::from_utf8(out.stdout)?
                            .trim()
                            .replace("Google Chrome ", "")
                            .replace("Google Chrome Canary ", "");
                        if !version.is_empty() {
                            return Ok(version);
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let commands = [
            ("google-chrome", "--version"),
            ("google-chrome-stable", "--version"),
            ("chromium", "--version"),
            ("chromium-browser", "--version"),
        ];

        for (cmd, arg) in commands {
            let output = Command::new(cmd)
                .arg(arg)
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    // "Google Chrome 143.0.1234.56" or "Chromium 143.0.1234.56"
                    let version_str = String::from_utf8(out.stdout)?;
                    let version = version_str
                        .trim()
                        .split_whitespace()
                        .last()
                        .unwrap_or("")
                        .to_string();
                    if !version.is_empty() && version.contains('.') {
                        return Ok(version);
                    }
                }
            }
        }
    }

    Err(anyhow!("Chrome is not installed or version cannot be verified.\nPlease install Chrome browser first."))
}

/// Parse download URL from Google JSON API
fn fetch_driver_url(major_version: &str) -> Result<String> {
    // Chrome for Testing latest versions API
    let api_url = "https://googlechromelabs.github.io/chrome-for-testing/latest-versions-per-milestone-with-downloads.json";
    
    let resp = reqwest::blocking::get(api_url)
        .map_err(|e| anyhow!("Failed to connect to Google API: {}", e))?
        .json::<ChromeVersions>()
        .map_err(|e| anyhow!("JSON parse failure: {}", e))?;

    let milestone = resp.milestones.get(major_version)
        .ok_or(anyhow!("No driver info for Chrome version ({}).\nBrowser might be too new or too old.", major_version))?;

    let artifacts = milestone.downloads.chromedriver.as_ref()
        .ok_or(anyhow!("Driver download list is empty."))?;

    // Select platform
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let platforms = ["win64", "win32"];
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    let platforms = ["win32"];
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let platforms = ["mac-arm64", "mac-x64"];
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let platforms = ["mac-x64"];
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    let platforms = ["linux64"];
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    let platforms = ["linux64"];  // No separate ARM64 Linux, try x64
    
    let artifact = platforms.iter()
        .find_map(|p| artifacts.iter().find(|a| a.platform == *p))
        .ok_or(anyhow!("Could not find driver URL for current OS/Architecture."))?;

    Ok(artifact.url.clone())
}

/// Download and extract ZIP
fn download_and_extract(url: &str, target_dir: &Path) -> Result<()> {
    println!("Starting driver download...");
    let resp = reqwest::blocking::get(url)?.bytes()?;
    let reader = Cursor::new(resp);
    
    let mut archive = zip::ZipArchive::new(reader)?;

    #[cfg(target_os = "windows")]
    let target_filename = "chromedriver.exe";
    #[cfg(not(target_os = "windows"))]
    let target_filename = "chromedriver";

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Extract only chromedriver from the archive
        if outpath.file_name().map(|s| s == target_filename).unwrap_or(false) {
            let dest_path = target_dir.join(target_filename);
            let mut outfile = fs::File::create(&dest_path)?;
            std::io::copy(&mut file, &mut outfile)?;
            
            // Grant execution permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&dest_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&dest_path, perms)?;
            }
            
            println!("Installation complete: {:?}", dest_path);
            return Ok(());
        }
    }

    Err(anyhow!("{} not found in downloaded archive.", target_filename))
}