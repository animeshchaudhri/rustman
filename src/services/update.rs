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
}

#[derive(serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(serde::Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

fn client() -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
    );
    if let Ok(token) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }
    reqwest::Client::builder()
        .user_agent(concat!("rustman/", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
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

    if !exe.exists() {
        return Err(format!(
            "Binary not found at {path}. The update replaced the file but the current \
             process may be reading a stale inode. Try running {path} manually.",
            path = exe.display(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, a downloaded binary is often quarantined.  Try to remove the
        // quarantine attribute so Gatekeeper doesn't block the relaunch.  If this
        // fails we still attempt to spawn — the error is propagated if it fails.
        let _ = std::process::Command::new("xattr")
            .args(["-dr", "com.apple.quarantine"])
            .arg(&exe)
            .output();
    }

    match std::process::Command::new(&exe).spawn() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            let hint = if cfg!(target_os = "macos") {
                "The binary may need code-signing or the quarantine flag removed. \
                 Try: xattr -dr com.apple.quarantine "
            } else if cfg!(target_os = "linux") {
                "If you are running via cargo run, the binary was swapped but the \
                 old inode may still be cached. Try running the binary directly: "
            } else {
                ""
            };
            Err(format!(
                "Failed to restart from {path}: {e}. {hint}{path}",
                path = exe.display(),
                hint = hint,
            ))
        }
    }
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

    // Sanity-check: the extracted binary should look like a valid executable.
    verify_binary(&tmp)?;

    self_replace::self_replace(&tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

/// Quick validation that a file at `path` is plausibly a native binary.
/// Checks the ELF / Mach-O / PE magic bytes.
fn verify_binary(path: &std::path::Path) -> Result<(), String> {
    let magic = std::fs::read(path)
        .map_err(|e| e.to_string())?;
    if magic.len() < 4 {
        return Err("extracted binary is too small".into());
    }
    match &magic[..4] {
        // ELF  (Linux)
        [0x7f, b'E', b'L', b'F'] => Ok(()),
        // Mach-O 64-bit  (macOS, fat binary)
        [0xcf, 0xfa, 0xed, 0xfe] | [0xca, 0xfe, 0xba, 0xbe] | [0xbe, 0xba, 0xfe, 0xca] => {
            Ok(())
        }
        // PE  (Windows)
        [b'M', b'Z', _, _] => Ok(()),
        h => Err(format!(
            "extracted binary has unknown header {h:02x?} — refusing to replace"
        )),
    }
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
