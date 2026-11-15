use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_REPO: &str = "gagan-gv/open-notch";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub html_url: String,
    pub body: String,
    pub deb_url: Option<String>,
    pub is_newer: bool,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate(String),
    UpdateAvailable(ReleaseInfo),
    Error(String),
}

/// Parses semantic version into (major, minor, patch)
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = clean.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch_part = parts.next().unwrap_or("0");
    let patch: u32 = patch_part.split('-').next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Checks whether `remote` is newer than `current`
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    match (parse_version(current), parse_version(remote)) {
        (Some(c), Some(r)) => r > c,
        _ => false,
    }
}

/// Simple JSON string value extractor for simple keys like "tag_name": "v0.1.1"
pub fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let idx = json.find(&pattern)?;
    let rest = json[idx + pattern.len()..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let mut out = String::new();
    let mut chars = rest.chars();
    let mut escaped = false;
    while let Some(c) = chars.next() {
        if escaped {
            match c {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                _ => {
                    out.push('\\');
                    out.push(c);
                }
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

/// Extracts any .deb asset download URL from the release JSON
pub fn extract_deb_url(json: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(idx) = json[search_from..].find("\"browser_download_url\":") {
        let abs_idx = search_from + idx;
        if let Some(url) = extract_json_string(&json[abs_idx..], "browser_download_url") {
            if url.ends_with(".deb") {
                return Some(url);
            }
        }
        search_from = abs_idx + 23;
    }
    None
}

/// Parse release info from JSON response
pub fn parse_release_json(json: &str, current_version: &str) -> Result<ReleaseInfo, String> {
    if json.contains("\"message\":") && json.contains("Not Found") {
        return Err("No public releases found yet on GitHub".to_string());
    }

    let tag_name = extract_json_string(json, "tag_name")
        .ok_or_else(|| "Could not parse tag_name from release JSON".to_string())?;

    let name = extract_json_string(json, "name").unwrap_or_else(|| tag_name.clone());
    let html_url = extract_json_string(json, "html_url").unwrap_or_else(|| {
        format!("https://github.com/{}/releases/tag/{}", GITHUB_REPO, tag_name)
    });
    let body = extract_json_string(json, "body").unwrap_or_default();
    let deb_url = extract_deb_url(json);
    let is_newer = is_newer_version(current_version, &tag_name);

    Ok(ReleaseInfo {
        tag_name,
        name,
        html_url,
        body,
        deb_url,
        is_newer,
    })
}

/// Path to cached last check file
fn get_cache_file_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|p| p.join("open-notch/last_update_check"))
}

/// Checks if at least `hours` have passed since last check
pub fn should_auto_check(hours: u64) -> bool {
    let path = match get_cache_file_path() {
        Some(p) => p,
        None => return true,
    };

    if let Ok(metadata) = std::fs::metadata(&path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed.as_secs() >= hours * 3600;
            }
        }
    }
    true
}

/// Records the current timestamp as last checked
pub fn record_check_timestamp() {
    if let Some(path) = get_cache_file_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        let _ = std::fs::write(path, now);
    }
}

/// Synchronously checks GitHub for the latest release (call from background thread)
pub fn fetch_latest_release(repo: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let output = Command::new("curl")
        .args([
            "-s",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "-H",
            "User-Agent: open-notch",
            "-H",
            "Accept: application/vnd.github.v3+json",
            &url,
        ])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err(format!("curl exited with status {}", output.status));
    }

    let body = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

    if body.trim().is_empty() {
        return Err("Empty response from GitHub".to_string());
    }

    Ok(body)
}

/// Perform update check synchronously and return `UpdateStatus`
pub fn check_updates_sync() -> UpdateStatus {
    record_check_timestamp();

    match fetch_latest_release(GITHUB_REPO) {
        Ok(json) => match parse_release_json(&json, CURRENT_VERSION) {
            Ok(info) => {
                if info.is_newer {
                    UpdateStatus::UpdateAvailable(info)
                } else {
                    UpdateStatus::UpToDate(CURRENT_VERSION.to_string())
                }
            }
            Err(e) => {
                // If no releases on GitHub yet, consider current version up to date
                if e.contains("No public releases found") {
                    UpdateStatus::UpToDate(CURRENT_VERSION.to_string())
                } else {
                    UpdateStatus::Error(e)
                }
            }
        },
        Err(e) => UpdateStatus::Error(e),
    }
}

/// Launch a URL in the user's default browser
pub fn open_url(url: &str) {
    let _ = Command::new("xdg-open").arg(url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(parse_version("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("v0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("V1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.1.0-beta"), Some((0, 1, 0)));
    }

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "1.0.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.1.1", "0.1.0"));
    }

    #[test]
    fn test_extract_json_string() {
        let sample = r#"{"tag_name":"v0.1.1","name":"OpenNotch Release","html_url":"https://github.com/test"}"#;
        assert_eq!(extract_json_string(sample, "tag_name"), Some("v0.1.1".to_string()));
        assert_eq!(extract_json_string(sample, "name"), Some("OpenNotch Release".to_string()));
        assert_eq!(extract_json_string(sample, "html_url"), Some("https://github.com/test".to_string()));
        assert_eq!(extract_json_string(sample, "non_existent"), None);
    }

    #[test]
    fn test_parse_release_json() {
        let sample = r#"{
            "tag_name": "v0.1.1",
            "name": "OpenNotch v0.1.1",
            "html_url": "https://github.com/gagan-gv/open-notch/releases/tag/v0.1.1",
            "body": "Fixed icons and added updates",
            "assets": [
                {
                    "name": "open-notch_0.1.1_amd64.deb",
                    "browser_download_url": "https://github.com/gagan-gv/open-notch/releases/download/v0.1.1/open-notch_0.1.1_amd64.deb"
                }
            ]
        }"#;

        let res = parse_release_json(sample, "0.1.0").unwrap();
        assert_eq!(res.tag_name, "v0.1.1");
        assert_eq!(res.name, "OpenNotch v0.1.1");
        assert!(res.is_newer);
        assert_eq!(
            res.deb_url,
            Some("https://github.com/gagan-gv/open-notch/releases/download/v0.1.1/open-notch_0.1.1_amd64.deb".to_string())
        );
    }
}
