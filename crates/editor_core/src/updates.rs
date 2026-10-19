//! Explicit release checks and verified Linux binary replacement. No automatic download.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Debug)]
pub struct Artifact {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub artifact: Option<Artifact>,
}
#[derive(Clone, Debug)]
pub enum Check {
    NoPublishedRelease,
    Current,
    Available(Release),
}
#[derive(Deserialize)]
struct Published {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}
pub fn repository(value: &str) -> Result<(), String> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
    {
        return Err("Use a GitHub repository in owner/name form".into());
    }
    Ok(())
}
fn asset_name() -> String {
    format!("heron-brush-linux-{}", std::env::consts::ARCH)
}
pub fn parse_release_list(
    bytes: &[u8],
    repo: &str,
    current: &str,
    include_pre: bool,
) -> Result<Check, String> {
    repository(repo)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Release metadata exceeds 2 MiB".into());
    }
    let current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    let releases: Vec<Published> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut eligible = releases
        .into_iter()
        .filter(|r| !r.draft && (include_pre || !r.prerelease))
        .filter_map(|r| {
            semver::Version::parse(r.tag_name.trim_start_matches('v'))
                .ok()
                .map(|version| (version, r))
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Ok(Check::NoPublishedRelease);
    }
    eligible.sort_by(|a, b| b.0.cmp(&a.0));
    let (version, release) = eligible.remove(0);
    if version <= current {
        return Ok(Check::Current);
    }
    let page_prefix = format!("https://github.com/{repo}/releases/");
    if !release.html_url.starts_with(&page_prefix) {
        return Err("Release page does not belong to the configured repository".into());
    }
    let download_prefix = format!("https://github.com/{repo}/releases/download/");
    let artifact = if cfg!(target_os = "linux") {
        release.assets.into_iter().find_map(|asset| {
            if asset.name != asset_name()
                || !asset.browser_download_url.starts_with(&download_prefix)
                || asset.size < 20
                || asset.size > 512 * 1024 * 1024
            {
                return None;
            }
            let hash = asset.digest?.strip_prefix("sha256:")?.to_owned();
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            Some(Artifact {
                url: asset.browser_download_url,
                sha256: hash.to_ascii_lowercase(),
                size: asset.size,
            })
        })
    } else {
        None
    };
    Ok(Check::Available(Release {
        version: version.to_string(),
        url: release.html_url,
        artifact,
    }))
}
fn client() -> Result<reqwest::blocking::Client, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::blocking::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("HeronBrush/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}
pub fn check(repo: &str, include_pre: bool) -> Result<Check, String> {
    repository(repo)?;
    let response = client()?
        .get(format!(
            "https://api.github.com/repos/{repo}/releases?per_page=30"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Check::NoPublishedRelease);
    }
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    let mut bytes = vec![];
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    parse_release_list(&bytes, repo, env!("CARGO_PKG_VERSION"), include_pre)
}
pub fn executable() -> Result<PathBuf, String> {
    if !cfg!(target_os = "linux") {
        return Err("Use the release downloads for this platform".into());
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return Err("Use release downloads to update an AppImage package".into());
    }
    std::env::current_exe().map_err(|e| e.to_string())
}
pub fn install(
    artifact: &Artifact,
    target: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64),
) -> Result<(), String> {
    let response = client()?
        .get(&artifact.url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    install_stream(response, artifact, target, cancel, progress)
}
fn install_stream(
    mut input: impl Read,
    artifact: &Artifact,
    target: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(u64, u64),
) -> Result<(), String> {
    let old = std::fs::metadata(target).map_err(|e| e.to_string())?;
    if !old.is_file() {
        return Err("The running application is not a replaceable file".into());
    }
    let pending = target.with_extension("update-pending");
    let backup = target.with_extension("update-backup");
    let mut created = false;
    let staged = (|| -> Result<(), String> {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending)
            .map_err(|e| e.to_string())?;
        created = true;
        let mut hash = Sha256::new();
        let mut count = 0;
        let mut bytes = [0; 65536];
        let mut header = vec![];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Update stopped".into());
            }
            let size = input.read(&mut bytes).map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            count += size as u64;
            if count > artifact.size {
                return Err("Update is larger than its published size".into());
            }
            if header.len() < 20 {
                header.extend_from_slice(&bytes[..size.min(20 - header.len())]);
            }
            hash.update(&bytes[..size]);
            output
                .write_all(&bytes[..size])
                .map_err(|e| e.to_string())?;
            progress(count, artifact.size);
        }
        let digest = hash
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if count != artifact.size || digest != artifact.sha256 {
            return Err(
                "Update size or SHA-256 digest does not match; application retained".into(),
            );
        }
        if header.len() < 20 || &header[..4] != b"\x7fELF" || header[4] != 2 || header[5] != 1 {
            return Err("The update is not a supported Linux executable".into());
        }
        let machine = u16::from_le_bytes([header[18], header[19]]);
        let expected = match std::env::consts::ARCH {
            "x86_64" => 62,
            "aarch64" => 183,
            _ => return Err("Automatic installation is unavailable on this architecture".into()),
        };
        if machine != expected {
            return Err("Update architecture does not match this application".into());
        }
        output.sync_all().map_err(|e| e.to_string())?;
        std::fs::set_permissions(&pending, old.permissions()).map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Update stopped".into());
        }
        if backup.exists() {
            std::fs::remove_file(&backup).map_err(|e| e.to_string())?;
        }
        std::fs::rename(target, &backup).map_err(|e| e.to_string())?;
        if let Err(error) = std::fs::rename(&pending, target) {
            std::fs::rename(&backup, target).map_err(|rollback| {
                format!(
                    "Install failed: {error}; restore failed: {rollback}. Backup: {}",
                    backup.display()
                )
            })?;
            return Err(error.to_string());
        }
        Ok(())
    })();
    if staged.is_err() && created {
        let _ = std::fs::remove_file(pending);
    }
    staged
}
#[cfg(test)]
mod tests {
    use super::*;
    fn payload() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        let machine: u16 = if std::env::consts::ARCH == "aarch64" {
            183
        } else {
            62
        };
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes
    }
    fn artifact(bytes: &[u8]) -> Artifact {
        Artifact {
            url: String::new(),
            size: bytes.len() as u64,
            sha256: Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        }
    }
    #[test]
    fn release_selection_excludes_drafts_prereleases_and_untrusted_assets() {
        let repo = "MojzisekJan/HeronBrush";
        let bytes=serde_json::to_vec(&serde_json::json!([
            {"tag_name":"v1.0.0","html_url":format!("https://github.com/{repo}/releases/tag/v1.0.0"),"assets":[]},
            {"tag_name":"v2.0.0-beta.1","html_url":"ignored","prerelease":true},
            {"tag_name":"v3.0.0","html_url":"ignored","draft":true}
        ])).unwrap();
        let Check::Available(release) =
            parse_release_list(&bytes, repo, "0.1.0-alpha.2", false).unwrap()
        else {
            panic!()
        };
        assert_eq!(release.version, "1.0.0");
        assert!(release.artifact.is_none());
        assert!(matches!(
            parse_release_list(b"[]", repo, "1.0.0", false).unwrap(),
            Check::NoPublishedRelease
        ));
        assert!(repository("../bad").is_err());
        assert!(parse_release_list(&bytes, repo, "0.1.0", true).is_err());
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn bad_digest_and_cancel_preserve_original_and_verified_install_keeps_backup() {
        let dir = std::env::temp_dir().join(format!("heron-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("heron-brush");
        std::fs::write(&target, b"original").unwrap();
        let bytes = payload();
        let mut bad = artifact(&bytes);
        bad.sha256 = "0".repeat(64);
        assert!(install_stream(
            bytes.as_slice(),
            &bad,
            &target,
            &AtomicBool::new(false),
            |_, _| {}
        )
        .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        let good = artifact(&bytes);
        assert!(install_stream(
            bytes.as_slice(),
            &good,
            &target,
            &AtomicBool::new(true),
            |_, _| {}
        )
        .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        let pending = target.with_extension("update-pending");
        std::fs::write(&pending, b"another operation").unwrap();
        assert!(install_stream(
            bytes.as_slice(),
            &good,
            &target,
            &AtomicBool::new(false),
            |_, _| {}
        )
        .is_err());
        assert_eq!(std::fs::read(&pending).unwrap(), b"another operation");
        std::fs::remove_file(&pending).unwrap();
        let mut wrong_arch = bytes.clone();
        wrong_arch[18..20].copy_from_slice(&0_u16.to_le_bytes());
        assert!(install_stream(
            wrong_arch.as_slice(),
            &artifact(&wrong_arch),
            &target,
            &AtomicBool::new(false),
            |_, _| {}
        )
        .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        install_stream(
            bytes.as_slice(),
            &good,
            &target,
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert_eq!(
            std::fs::read(target.with_extension("update-backup")).unwrap(),
            b"original"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
