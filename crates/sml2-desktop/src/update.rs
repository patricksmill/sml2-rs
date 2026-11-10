//! In-game self-update: release lookup, checksum-verified download and install.
//!
//! The network and filesystem work runs on detached threads; the UI only polls
//! [`Updater`]'s shared phase. All pure rules (version order, asset names,
//! checksum lines and the daily policy) are kept free of I/O so they stay
//! testable without a network or a release build.

use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &str = "https://github.com/patricksmill/sml2-rs";
pub const RELEASES_PAGE: &str = "https://github.com/patricksmill/sml2-rs/releases";
pub const GETTING_STARTED: &str =
    "https://github.com/patricksmill/sml2-rs/blob/main/docs/Getting-Started.md";
pub const ISSUES: &str = "https://github.com/patricksmill/sml2-rs/issues/new/choose";
pub const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/patricksmill/sml2-rs/releases/latest";
pub const USER_AGENT: &str = concat!(
    "SixGoldenCoins/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/patricksmill/sml2-rs)"
);

/// Automatic checks are limited to one per day.
pub const CHECK_INTERVAL: u64 = 24 * 60 * 60;

/// Official release builds opt in at compile time; everything else (including
/// `cargo run` and source builds) never offers or installs updates.
pub const RELEASE_BUILD: bool = option_env!("SML2_RELEASE_BUILD").is_some();

pub const CHECKSUM_ASSET: &str = "SHA256SUMS.txt";

const ASSET_PREFIX: &str = "SixGoldenCoins";

/// A release version, compared numerically without a semver dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        let core = raw
            .strip_prefix(['v', 'V'])
            .unwrap_or(raw)
            .split(['-', '+'])
            .next()?;
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn current_version() -> Version {
    Version::parse(VERSION).unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
    })
}

/// The release tag written with the conventional leading `v`.
pub fn normalize_tag(tag: &str) -> String {
    let tag = tag.trim();
    if tag.starts_with(['v', 'V']) {
        format!("v{}", tag.trim_start_matches(['v', 'V']))
    } else {
        format!("v{tag}")
    }
}

pub fn release_notes_url() -> String {
    format!("{REPOSITORY}/releases/tag/v{VERSION}")
}

/// The three release assets the updater can install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    LinuxX86_64,
    WindowsX64,
    MacosArm64,
}

impl Platform {
    pub fn current() -> Option<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Some(Self::LinuxX86_64)
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Some(Self::WindowsX64)
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Some(Self::MacosArm64)
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
        )))]
        {
            None
        }
    }

    /// `version` is the numeric version, without the leading `v`.
    pub fn asset_name(self, version: Version) -> String {
        let tag = format!("v{version}");
        match self {
            Self::LinuxX86_64 => format!("{ASSET_PREFIX}-{tag}-linux-x86_64.AppImage"),
            Self::WindowsX64 => format!("{ASSET_PREFIX}-{tag}-windows-x64.zip"),
            Self::MacosArm64 => format!("{ASSET_PREFIX}-{tag}-macos-arm64.zip"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Normalized tag, e.g. `v0.2.2`.
    pub tag: String,
    pub version: Version,
    /// First body line; the player-facing update notice.
    pub summary: String,
    pub url: String,
    pub assets: Vec<Asset>,
}

pub fn parse_release(body: &str) -> Result<Release, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|error| format!("GitHub sent an unreadable reply: {error}"))?;
    let tag = value
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or("The release has no tag")?;
    let version = Version::parse(tag).ok_or_else(|| format!("Unsupported release tag {tag}"))?;
    let summary = value
        .get("body")
        .and_then(serde_json::Value::as_str)
        .and_then(|body| body.lines().next())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or("A new version is available.")
        .to_string();
    let url = value
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    let assets = value
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .map(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    Some(Asset {
                        name: asset.get("name")?.as_str()?.to_string(),
                        url: asset.get("browser_download_url")?.as_str()?.to_string(),
                        size: asset.get("size").and_then(serde_json::Value::as_u64),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release {
        tag: normalize_tag(tag),
        version,
        summary,
        url,
        assets,
    })
}

pub fn asset_for(assets: &[Asset], platform: Platform, version: Version) -> Option<&Asset> {
    let name = platform.asset_name(version);
    assets.iter().find(|asset| asset.name == name)
}

fn checksum_asset(release: &Release) -> Option<&Asset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == CHECKSUM_ASSET)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Find one `<64 hex><two spaces><filename>` line in `SHA256SUMS.txt`.
pub fn checksum_for<'a>(text: &'a str, asset: &str) -> Option<&'a str> {
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        let Some((hash, name)) = line.split_once("  ") else {
            continue;
        };
        if name == asset && is_sha256_hex(hash) {
            return Some(hash);
        }
    }
    None
}

pub fn verify_sha256(bytes: &[u8], expected: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(&hasher.finalize()) == expected.to_ascii_lowercase()
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// Whether an automatic check is due. `now` and `last` are Unix seconds.
pub fn auto_check_due(enabled: bool, last_check: Option<u64>, now: u64) -> bool {
    enabled && last_check.is_none_or(|last| now.saturating_sub(last) >= CHECK_INTERVAL)
}

/// A player's skipped release stays ignored until a newer one appears.
pub fn should_offer(version: Version, skipped: Option<&str>) -> bool {
    let Some(skipped) = skipped else {
        return true;
    };
    match Version::parse(skipped) {
        Some(skipped) => version > skipped,
        None => normalize_tag(skipped) != format!("v{version}"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    /// A newer release is available and has not been skipped.
    Available,
    Downloading {
        received: u64,
        total: Option<u64>,
    },
    /// Verified and staged; `Restart to update` installs it.
    Ready,
    UpToDate,
    /// A source build asked for updates; only official releases have them.
    SourceBuild,
    Failed(String),
}

struct Pending {
    file: PathBuf,
    target: Target,
    release: Release,
}

struct Shared {
    phase: Phase,
    release: Option<Release>,
    manual: bool,
    pending: Option<Pending>,
}

fn lock(shared: &Arc<Mutex<Shared>>) -> MutexGuard<'_, Shared> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Background update state owned by the desktop app.
pub struct Updater {
    shared: Arc<Mutex<Shared>>,
}

impl Default for Updater {
    fn default() -> Self {
        Self::new()
    }
}

impl Updater {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(Shared {
                phase: Phase::Idle,
                release: None,
                manual: false,
                pending: None,
            })),
        }
    }

    pub fn phase(&self) -> Phase {
        lock(&self.shared).phase.clone()
    }

    pub fn release(&self) -> Option<Release> {
        lock(&self.shared).release.clone()
    }

    pub fn is_manual(&self) -> bool {
        lock(&self.shared).manual
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            lock(&self.shared).phase,
            Phase::Checking | Phase::Downloading { .. }
        )
    }

    pub fn is_ready(&self) -> bool {
        lock(&self.shared).phase == Phase::Ready
    }

    /// Start a check. Manual checks ignore the daily limit and skipped version.
    pub fn check(&mut self, manual: bool, skipped_version: Option<String>) {
        {
            let mut shared = lock(&self.shared);
            if matches!(shared.phase, Phase::Checking | Phase::Downloading { .. }) {
                return;
            }
            if !RELEASE_BUILD {
                if manual {
                    shared.manual = true;
                    shared.release = None;
                    shared.pending = None;
                    shared.phase = Phase::SourceBuild;
                }
                return;
            }
            shared.manual = manual;
            if shared.pending.is_some() {
                // A verified download is already waiting; offer it again
                // instead of discarding it.
                shared.phase = Phase::Ready;
                return;
            }
            shared.release = None;
            shared.phase = Phase::Checking;
        }
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            let result = fetch_latest();
            let mut state = lock(&shared);
            match result {
                Ok(release) => {
                    let newer = release.version > current_version();
                    let skipped = !manual
                        && skipped_version
                            .as_deref()
                            .is_some_and(|skipped| !should_offer(release.version, Some(skipped)));
                    state.phase = if newer && !skipped {
                        Phase::Available
                    } else if manual && !newer {
                        Phase::UpToDate
                    } else {
                        Phase::Idle
                    };
                    state.release = Some(release);
                }
                Err(_) if !manual => {
                    // Automatic network failures are silent.
                    state.phase = Phase::Idle;
                    state.release = None;
                }
                Err(error) => {
                    state.phase = Phase::Failed(error);
                    state.release = None;
                }
            }
        });
    }

    /// Download, verify and stage the release found by the last check.
    pub fn download(&mut self) {
        let release = {
            let mut shared = lock(&self.shared);
            if shared.phase != Phase::Available {
                return;
            }
            let Some(release) = shared.release.clone() else {
                return;
            };
            shared.phase = Phase::Downloading {
                received: 0,
                total: None,
            };
            release
        };
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            let result = stage_update(&shared, &release);
            let mut state = lock(&shared);
            match result {
                Ok(pending) => {
                    state.pending = Some(pending);
                    state.phase = Phase::Ready;
                }
                Err(Failure::OpenPage(message)) => {
                    open_url(&release.url);
                    state.phase = Phase::Failed(format!("{message} Opening the download page."));
                }
                Err(Failure::Report(message)) => state.phase = Phase::Failed(message),
            }
        });
    }

    /// Hide the notice but keep a verified download for install on exit.
    pub fn dismiss(&mut self) {
        let mut shared = lock(&self.shared);
        shared.phase = Phase::Idle;
        shared.manual = false;
    }

    /// Skip the release currently on offer; returns its tag for persistence.
    pub fn skip(&mut self) -> Option<String> {
        let mut shared = lock(&self.shared);
        let tag = shared.release.as_ref()?.tag.clone();
        shared.phase = Phase::Idle;
        shared.manual = false;
        Some(tag)
    }

    /// Apply the staged update and relaunch. Returns whether the app should exit.
    pub fn install_and_restart(&mut self) -> bool {
        let pending = lock(&self.shared).pending.take();
        let Some(pending) = pending else {
            return false;
        };
        match apply(&pending, true) {
            Ok(()) => {
                let mut shared = lock(&self.shared);
                shared.phase = Phase::Idle;
                shared.manual = false;
                shared.release = None;
                true
            }
            Err(error) => {
                open_url(&pending.release.url);
                let mut shared = lock(&self.shared);
                shared.phase = Phase::Failed(format!(
                    "The update could not be installed: {error}. Opening the download page."
                ));
                false
            }
        }
    }

    /// Install a staged update as the app quits, without relaunching.
    pub fn apply_on_exit(&mut self) {
        let pending = lock(&self.shared).pending.take();
        if let Some(pending) = pending {
            if let Err(error) = apply(&pending, false) {
                eprintln!("The update could not be installed: {error}");
                open_url(&pending.release.url);
            }
        }
    }
}

/// Remove any old executable left by a manual Windows replacement.
pub fn remove_previous_executable() {
    #[cfg(windows)]
    if let Ok(exe) = env::current_exe() {
        if let Some(directory) = exe.parent() {
            let _ = fs::remove_file(directory.join("sml2.exe.old"));
        }
    }
}

enum Failure {
    /// The install location cannot be written; hand the player to the browser.
    OpenPage(String),
    /// A download or checksum problem to report in place.
    Report(String),
}

fn install_target() -> Result<Target, String> {
    #[cfg(windows)]
    {
        let exe = env::current_exe().map_err(|error| error.to_string())?;
        let directory = exe
            .parent()
            .ok_or("The game executable has no folder")?
            .to_path_buf();
        Ok(Target::WindowsExe { directory })
    }
    #[cfg(target_os = "linux")]
    {
        let path = env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .ok_or("This copy of the game is not running from an AppImage")?;
        let directory = path
            .parent()
            .ok_or("The AppImage has no folder")?
            .to_path_buf();
        Ok(Target::AppImage { path, directory })
    }
    #[cfg(target_os = "macos")]
    {
        let exe = env::current_exe().map_err(|error| error.to_string())?;
        let bundle = exe
            .ancestors()
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
            })
            .ok_or("The game is not inside an application bundle")?
            .to_path_buf();
        let directory = bundle
            .parent()
            .ok_or("The application bundle has no folder")?
            .to_path_buf();
        Ok(Target::MacBundle { bundle, directory })
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Err("Updates are not available for this platform".into())
    }
}

enum Target {
    #[cfg(windows)]
    WindowsExe { directory: PathBuf },
    #[cfg(target_os = "linux")]
    AppImage { path: PathBuf, directory: PathBuf },
    #[cfg(target_os = "macos")]
    MacBundle { bundle: PathBuf, directory: PathBuf },
}

impl Target {
    fn directory(&self) -> &Path {
        match self {
            #[cfg(windows)]
            Self::WindowsExe { directory } => directory,
            #[cfg(target_os = "linux")]
            Self::AppImage { directory, .. } => directory,
            #[cfg(target_os = "macos")]
            Self::MacBundle { directory, .. } => directory,
        }
    }
}

fn stage_update(shared: &Arc<Mutex<Shared>>, release: &Release) -> Result<Pending, Failure> {
    let platform = Platform::current()
        .ok_or_else(|| Failure::OpenPage("Updates are not available for this platform.".into()))?;
    let target = install_target().map_err(Failure::OpenPage)?;
    let asset = asset_for(&release.assets, platform, release.version)
        .ok_or_else(|| Failure::OpenPage("This release has no build for this platform.".into()))?;
    let sums = checksum_asset(release)
        .ok_or_else(|| Failure::Report("This release is missing its checksum file.".into()))?;

    let file_name = format!(
        ".sml2-update-{}-{}.tmp",
        release.version,
        std::process::id()
    );
    let file = target.directory().join(file_name);
    let result = stage_download(shared, asset, sums, &file);
    match result {
        Ok(()) => Ok(Pending {
            file,
            target,
            release: release.clone(),
        }),
        Err(error) => {
            let _ = fs::remove_file(&file);
            Err(error)
        }
    }
}

fn stage_download(
    shared: &Arc<Mutex<Shared>>,
    asset: &Asset,
    sums: &Asset,
    file: &Path,
) -> Result<(), Failure> {
    let mut output = File::create(file).map_err(|error| {
        Failure::OpenPage(format!("The game folder is not writable ({error})."))
    })?;
    let digest = download(shared, &asset.url, &mut output, asset.size).map_err(Failure::Report)?;
    output
        .flush()
        .map_err(|error| Failure::Report(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).map_err(|error| {
            Failure::OpenPage(format!(
                "The update could not be made executable ({error})."
            ))
        })?;
    }

    let sums_text = fetch_text(&sums.url).map_err(Failure::Report)?;
    let expected = checksum_for(&sums_text, &asset.name).ok_or_else(|| {
        Failure::Report(format!("{} is missing from the checksum file.", asset.name))
    })?;
    if digest != expected.to_ascii_lowercase() {
        return Err(Failure::Report(
            "The update download did not match its published checksum.".into(),
        ));
    }
    Ok(())
}

fn download(
    shared: &Arc<Mutex<Shared>>,
    url: &str,
    output: &mut File,
    size: Option<u64>,
) -> Result<String, String> {
    let mut response = request(url)?;
    let total = response.body().content_length().or(size);
    {
        let mut state = lock(shared);
        state.phase = Phase::Downloading { received: 0, total };
    }
    let mut reader = response.body_mut().as_reader();
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut received = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("The update download stopped: {error}"))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("The update could not be saved: {error}"))?;
        hasher.update(&buffer[..read]);
        received += read as u64;
        let mut state = lock(shared);
        state.phase = Phase::Downloading { received, total };
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn request(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("Could not reach GitHub: {error}"))
}

fn fetch_text(url: &str) -> Result<String, String> {
    let mut response = request(url)?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Could not read GitHub's reply: {error}"))
}

fn fetch_latest() -> Result<Release, String> {
    let body = fetch_text(LATEST_RELEASE_API)?;
    parse_release(&body)
}

fn apply(pending: &Pending, relaunch: bool) -> Result<(), String> {
    match &pending.target {
        #[cfg(windows)]
        Target::WindowsExe { directory } => {
            let executable = env::current_exe().map_err(|error| error.to_string())?;
            let new = directory.join(format!(".sml2-new-{}.exe", std::process::id()));
            let old = directory.join("sml2.exe.old");
            extract_windows_executable(&pending.file, &new)?;
            // Windows allows renaming a running executable but not replacing it.
            let _ = fs::remove_file(&old);
            fs::rename(&executable, &old)
                .map_err(|error| format!("The running copy could not be moved aside: {error}"))?;
            if let Err(error) = fs::rename(&new, &executable) {
                let _ = fs::rename(&old, &executable);
                let _ = fs::remove_file(&new);
                return Err(format!("The new copy could not be put in place: {error}"));
            }
            let _ = fs::remove_file(&pending.file);
            if relaunch {
                let exe = executable;
                Command::new(exe)
                    .spawn()
                    .map_err(|error| format!("The new copy could not be started: {error}"))?;
            }
        }
        #[cfg(target_os = "linux")]
        Target::AppImage { path, .. } => {
            fs::rename(&pending.file, path)
                .map_err(|error| format!("The AppImage could not be replaced: {error}"))?;
            if relaunch {
                Command::new(path)
                    .spawn()
                    .map_err(|error| format!("The new AppImage could not be started: {error}"))?;
            }
        }
        #[cfg(target_os = "macos")]
        Target::MacBundle { bundle, directory } => {
            let unpack = directory.join(format!(".sml2-unpack-{}", std::process::id()));
            let _ = fs::remove_dir_all(&unpack);
            fs::create_dir_all(&unpack).map_err(|error| error.to_string())?;
            let extracted = extract_app(&pending.file, &unpack)?;
            let old = directory.join(format!(".sml2-old-{}.app", std::process::id()));
            let _ = fs::remove_dir_all(&old);
            fs::rename(bundle, &old)
                .map_err(|error| format!("The application could not be replaced: {error}"))?;
            if let Err(error) = fs::rename(&extracted, bundle) {
                let _ = fs::rename(&old, bundle);
                let _ = fs::remove_dir_all(&unpack);
                return Err(format!(
                    "The new application could not be moved in: {error}"
                ));
            }
            let _ = fs::remove_dir_all(&old);
            let _ = fs::remove_dir_all(&unpack);
            let _ = fs::remove_file(&pending.file);
            if relaunch {
                Command::new("open")
                    .arg("-n")
                    .arg(bundle)
                    .spawn()
                    .map_err(|error| {
                        format!("The new application could not be started: {error}")
                    })?;
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn extract_windows_executable(zip: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(zip).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("The update archive is unreadable: {error}"))?;
    let mut entry = archive
        .by_name("sml2.exe")
        .map_err(|_| "The update archive does not contain sml2.exe".to_string())?;
    let mut output = File::create(destination).map_err(|error| error.to_string())?;
    std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn extract_app(zip: &Path, directory: &Path) -> Result<PathBuf, String> {
    let file = File::open(zip).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("The update archive is unreadable: {error}"))?;
    archive
        .extract(directory)
        .map_err(|error| format!("The update archive could not be unpacked: {error}"))?;
    fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        })
        .ok_or_else(|| "The update archive does not contain an application bundle".into())
}

/// Open a URL with the platform's browser, never showing an elevation prompt.
pub fn open_url(url: &str) {
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd");
        command.args(["/c", "start", "", url]);
        let _ = command.spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(not(any(windows, unix)))]
    let _ = url;
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUMS: &str = "\
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  SixGoldenCoins-v0.2.2-linux-x86_64.AppImage
abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  other.zip
";

    #[test]
    fn versions_compare_numerically_and_ignore_the_leading_v() {
        assert_eq!(Version::parse("v0.2.1"), Version::parse("0.2.1"));
        assert!(Version::parse("v0.10.0").unwrap() > Version::parse("v0.9.9").unwrap());
        assert!(Version::parse("V1.0.0").unwrap() > Version::parse("0.99.99").unwrap());
        assert!(!(Version::parse("v0.2.1").unwrap() > Version::parse("v0.2.1").unwrap()));
        assert!(Version::parse("v0.2.1-rc1").is_some());
        assert!(Version::parse("nightly").is_none());
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.2.3.4").is_none());
    }

    #[test]
    fn asset_names_match_the_release_contract_per_platform() {
        let version = Version::parse("0.2.2").unwrap();
        assert_eq!(
            Platform::LinuxX86_64.asset_name(version),
            "SixGoldenCoins-v0.2.2-linux-x86_64.AppImage"
        );
        assert_eq!(
            Platform::WindowsX64.asset_name(version),
            "SixGoldenCoins-v0.2.2-windows-x64.zip"
        );
        assert_eq!(
            Platform::MacosArm64.asset_name(version),
            "SixGoldenCoins-v0.2.2-macos-arm64.zip"
        );
        let assets = vec![
            Asset {
                name: Platform::LinuxX86_64.asset_name(version),
                url: "linux".into(),
                size: None,
            },
            Asset {
                name: Platform::WindowsX64.asset_name(version),
                url: "windows".into(),
                size: None,
            },
        ];
        assert_eq!(
            asset_for(&assets, Platform::WindowsX64, version)
                .unwrap()
                .url,
            "windows"
        );
        assert!(asset_for(&assets, Platform::MacosArm64, version).is_none());
    }

    #[test]
    fn release_json_keeps_the_first_body_line_and_asset_list() {
        let json = r#"{
            "tag_name": "v0.2.2",
            "body": "Faster title screen\n\nLonger notes here.",
            "html_url": "https://example.invalid/tag",
            "assets": [
                {"name": "SixGoldenCoins-v0.2.2-linux-x86_64.AppImage",
                 "browser_download_url": "https://example.invalid/appimage",
                 "size": 42},
                {"name": "SHA256SUMS.txt",
                 "browser_download_url": "https://example.invalid/sums",
                 "size": 100}
            ]
        }"#;
        let release = parse_release(json).unwrap();
        assert_eq!(release.tag, "v0.2.2");
        assert_eq!(release.version, Version::parse("0.2.2").unwrap());
        assert_eq!(release.summary, "Faster title screen");
        assert_eq!(release.assets.len(), 2);
        assert_eq!(release.assets[0].size, Some(42));
    }

    #[test]
    fn checksum_lines_are_found_and_mismatches_fail_verification() {
        assert_eq!(
            checksum_for(SUMS, "SixGoldenCoins-v0.2.2-linux-x86_64.AppImage"),
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        );
        assert!(checksum_for(SUMS, "missing.zip").is_none());

        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_sha256(b"abc", expected));
        let mut mismatched = expected.to_string();
        mismatched.replace_range(..1, "c");
        assert!(!verify_sha256(b"abc", &mismatched));
        assert!(!verify_sha256(b"abd", expected));
    }

    #[test]
    fn automatic_checks_are_daily_and_skipped_versions_stay_quiet() {
        assert!(auto_check_due(true, None, 100));
        assert!(!auto_check_due(false, None, 100));
        assert!(!auto_check_due(true, Some(100), 100 + CHECK_INTERVAL - 1));
        assert!(auto_check_due(true, Some(100), 100 + CHECK_INTERVAL));
        assert!(!auto_check_due(true, Some(1000), 100));

        let release = Version::parse("0.2.2").unwrap();
        assert!(should_offer(release, None));
        assert!(!should_offer(release, Some("0.2.2")));
        assert!(!should_offer(release, Some("v0.2.2")));
        assert!(!should_offer(release, Some("0.2.3")));
        assert!(should_offer(
            Version::parse("0.3.0").unwrap(),
            Some("0.2.2")
        ));
        assert!(should_offer(release, Some("not-a-version")));
    }

    #[test]
    fn release_notes_point_at_the_running_version() {
        assert_eq!(
            release_notes_url(),
            format!("{REPOSITORY}/releases/tag/v{VERSION}")
        );
    }
}
