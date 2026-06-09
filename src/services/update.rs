//! In-app self-update from GitHub releases (https://github.com/animeshchaudhri/rustman).
//!
//! Uses the app's existing async `reqwest` to fetch the latest release and the
//! matching platform asset, then extracts the binary and atomically swaps the
//! running executable. The asset tokens / binary name must match the artefacts
//! produced by `.github/workflows/release.yml`.

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/animeshchaudhri/rustman/releases/latest";

/// Substring matched against the release asset names for the running platform.
#[cfg(target_os = "macos")]
const ASSET_TARGET: &str = "macos-universal";
#[cfg(target_os = "linux")]
const ASSET_TARGET: &str = "linux-x86_64";
#[cfg(target_os = "windows")]
const ASSET_TARGET: &str = "windows-x86_64";

/// Name of the executable inside the downloaded archive.
#[cfg(not(target_os = "windows"))]
const BIN_NAME: &str = "rustman";
#[cfg(target_os = "windows")]
const BIN_NAME: &str = "rustman.exe";

/// The current build's version, baked in at compile time.
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// The newer version available, e.g. `"0.4.0"`.
    pub version: String,
    /// The running version.
    pub current: String,
    /// Release notes body (may be empty).
    pub notes: String,
}

#[derive(serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

fn client() -> Result<reqwest::Client, String> {
    // GitHub's API rejects requests without a User-Agent.
    reqwest::Client::builder()
        .user_agent(concat!("rustman/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_latest() -> Result<GhRelease, String> {
    client()?
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<GhRelease>()
        .await
        .map_err(|e| e.to_string())
}

/// Query the latest release; returns `Some` only when it is newer than the
/// running build.
pub async fn check() -> Result<Option<UpdateInfo>, String> {
    let release = fetch_latest().await?;
    let latest = release.tag_name.trim_start_matches('v').to_owned();
    let current = current_version();
    Ok(version_gt(&latest, current).then(|| UpdateInfo {
        version: latest,
        current: current.to_owned(),
        notes: release.body.unwrap_or_default(),
    }))
}

pub async fn install() -> Result<String, String> {
    let release = fetch_latest().await?;
    let latest = release.tag_name.trim_start_matches('v').to_owned();
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.contains(ASSET_TARGET))
        .ok_or_else(|| format!("release {latest} has no '{ASSET_TARGET}' asset"))?;
    let bytes = client()?
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?
        .to_vec();
    tokio::task::spawn_blocking(move || extract_and_replace(bytes))
        .await
        .map_err(|e| e.to_string())??;
    Ok(latest)
}

/// Relaunch the freshly-updated executable and exit the current process.
pub fn restart() -> Result<std::convert::Infallible, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::process::exit(0);
}

/// Compare two `MAJOR.MINOR.PATCH` strings (pre-release suffixes are ignored).
fn version_gt(a: &str, b: &str) -> bool {
    parse_ver(a) > parse_ver(b)
}

fn parse_ver(v: &str) -> (u64, u64, u64) {
    let core = v.split(['-', '+']).next().unwrap_or(v);
    let mut it = core.split('.').map(|x| x.trim().parse().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

/// Unpack the platform binary from the downloaded archive and swap it in for the
/// running executable. Blocking (filesystem + decompression).
fn extract_and_replace(bytes: Vec<u8>) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = current.parent().ok_or("current exe has no parent dir")?;
    let tmp = dir.join(".rustman-update.tmp");

    extract_binary(&bytes, &tmp)?;
    self_replace::self_replace(&tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_binary(bytes: &[u8], out: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let is_bin = entry
            .path()
            .map(|p| p.file_name() == Some(std::ffi::OsStr::new(BIN_NAME)))
            .unwrap_or(false);
        if is_bin {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            std::fs::write(out, &buf).map_err(|e| e.to_string())?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(out, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("'{BIN_NAME}' not found in archive"))
}

#[cfg(target_os = "windows")]
fn extract_binary(bytes: &[u8], out: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.name().rsplit(['/', '\\']).next() == Some(BIN_NAME) {
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            std::fs::write(out, &buf).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("'{BIN_NAME}' not found in archive"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(version_gt("0.4.0", "0.3.2"));
        assert!(version_gt("1.0.0", "0.9.9"));
        assert!(version_gt("0.3.10", "0.3.2"));
        assert!(!version_gt("0.3.2", "0.3.2"));
        assert!(!version_gt("0.3.1", "0.3.2"));
        // pre-release suffix is ignored, so a dev tag of the current version
        // is not treated as an upgrade.
        assert!(!version_gt("0.3.2-dev.5", "0.3.2"));
    }
}
