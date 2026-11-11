use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use color_eyre::eyre::{Result, eyre};
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// Whether this is a local build (`HEX_LOCAL_BUILD=1`): made from a checkout
/// for its owner's own machine, so it never checks for or offers updates.
pub fn is_local_build() -> bool {
    option_env!("HEX_LOCAL_BUILD") == Some("1")
}

/// Whether this is a macOS `Hex Test.app` build (`HEX_TEST_BUILD=1`), which
/// keeps its data, settings, and user config apart from the installed Hex.
pub fn is_macos_test_build() -> bool {
    cfg!(target_os = "macos") && option_env!("HEX_TEST_BUILD") == Some("1")
}

/// The name menus show, so a test build running beside Hex reads as itself.
#[cfg(target_os = "macos")]
pub fn app_name() -> &'static str {
    if is_macos_test_build() {
        "Hex Test"
    } else {
        "Hex"
    }
}

/// The file that names a chosen data root. It lives beside the executable's
/// install folder (not inside the movable data), so moving the data never
/// hides the pointer to it. A missing file means the default root.
#[cfg(windows)]
const DATA_ROOT_FILE: &str = "hex-data-root.txt";

/// A chosen data root read before anything else, so `support_dir` can find
/// `settings.json` without loading settings. `None` keeps the default.
pub fn chosen_data_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let root = this_data_root()?;
        let path = root.join(DATA_ROOT_FILE);
        let text = fs::read_to_string(path).ok()?;
        let chosen = PathBuf::from(text.trim());
        (!chosen.as_os_str().is_empty()).then_some(chosen)
    }
    #[cfg(not(windows))]
    {
        // On macOS and Linux the data root stays the platform folder; the
        // window offers this choice only where Hex owns the install folder.
        None
    }
}

/// Where the data-root pointer is written when a person chooses a folder.
#[cfg(windows)]
pub fn data_root_file() -> Result<PathBuf> {
    Ok(this_data_root()
        .ok_or_else(|| eyre!("this executable has no install folder"))?
        .join(DATA_ROOT_FILE))
}

/// Moves everything Hex has written to `new` and points future launches
/// there. The current process keeps its old paths (log handles, locks, and
/// cached folders), so the window asks for a restart. `move_once` decides
/// between a rename and a cross-volume copy.
#[cfg(windows)]
pub fn set_data_root(new: &Path) -> Result<()> {
    let current = support_dir()?;
    if current == new {
        return Ok(());
    }
    relocate_data(&current, new)?;
    let file = data_root_file()?;
    let partial = file.with_extension("txt.partial");
    fs::write(&partial, new.to_string_lossy().as_bytes())?;
    fs::rename(&partial, &file)?;
    Ok(())
}

/// Copies or moves Hex's data folders from `current` to `new`, leaving the
/// install's own files (versions, bin, receipt) where the executable runs.
#[cfg(windows)]
fn relocate_data(current: &Path, new: &Path) -> Result<()> {
    fs::create_dir_all(new)?;
    // A file the running process holds open (the process log, the local API
    // lock) is copied instead of moved; the old copy stays behind and the
    // new one takes over after the restart.
    for name in [
        "settings.json",
        "history.json",
        "logs",
        "models",
        "captures",
        "meetings",
        "updates",
        "local-api.json",
        "update-bucket",
        "onboarding-complete",
        "personal-commands.json",
        "transform-trust.json",
    ] {
        let from = current.join(name);
        if !from.exists() {
            continue;
        }
        let to = new.join(name);
        if to.exists() {
            continue;
        }
        if fs::rename(&from, &to).is_err() && move_across(&from, &to).is_err() {
            let copied = if from.is_dir() {
                copy_dir(&from, &to)
            } else {
                fs::copy(&from, &to).map(|_| ())
            };
            copied.map_err(|error| {
                eyre!(
                    "could not copy {} to {}: {error}",
                    from.display(),
                    to.display()
                )
            })?;
        }
    }
    Ok(())
}

pub fn support_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("HEX_APPLICATION_SUPPORT_DIR") {
        return Ok(path.into());
    }
    if let Some(root) = chosen_data_root() {
        return Ok(root);
    }
    #[cfg(windows)]
    if let Some(root) = this_data_root() {
        // Hex keeps everything it produces — settings, history, logs,
        // models, downloads — beside the running executable, whether it is
        // an installed version or a loose build.
        return Ok(root);
    }
    let data =
        dirs::data_dir().ok_or_else(|| eyre!("application data directory is unavailable"))?;
    let folder = if is_macos_test_build() {
        "hex-test"
    } else {
        "hex"
    };
    Ok(data.join(folder))
}

/// The folder holding this executable's `versions\<version>\` layout: the
/// managed install's root. `None` unless the executable sits two folders
/// below one named `versions`, so a loose or copied `hex.exe` — a build in
/// `target\debug`, a download — never claims to be an install the updater
/// can replace (but its data still lives beside it, see `data_root`).
#[cfg(windows)]
pub fn managed_root(executable: &Path) -> Option<PathBuf> {
    let versions = executable.parent()?.parent()?;
    if !versions.file_name()?.eq_ignore_ascii_case("versions") {
        return None;
    }
    Some(versions.parent()?.to_path_buf())
}

/// The folder that holds this executable's data: an installed version's
/// root, or the folder the executable itself sits in for a loose build.
/// Hex writes nothing outside this folder on Windows.
#[cfg(windows)]
pub fn data_root(executable: &Path) -> Option<PathBuf> {
    managed_root(executable).or_else(|| Some(executable.parent()?.to_path_buf()))
}

/// [`data_root`] for the running executable.
#[cfg(windows)]
fn this_data_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| data_root(&executable))
}

pub fn logs_dir() -> Result<PathBuf> {
    static LOGS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    machine_local(&LOGS, "logs")
}

/// Where the speech models live (`transcription::models::models_dir`).
pub fn models_dir() -> Result<PathBuf> {
    static MODELS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    machine_local(&MODELS, "models")
}

/// `name` beside the executable's data root, adopting an older Hex's
/// `%LOCALAPPDATA%\hex` logs and models once — so models never download
/// again after the update. While a move can't happen (a running Hex holds
/// its files), the old copy keeps being used, so nothing downloads again.
fn machine_local(cell: &std::sync::OnceLock<PathBuf>, name: &str) -> Result<PathBuf> {
    if let Some(path) = cell.get() {
        return Ok(path.clone());
    }
    let beside = support_dir()?.join(name);
    let path = match local_support_dir() {
        Some(local) => move_once(&beside, &local.join(name)),
        None => adopt_machine_local(&beside, name),
    };
    Ok(cell.get_or_init(|| path).clone())
}

/// `%LOCALAPPDATA%\hex` on non-Windows and on Windows where a data folder
/// is chosen for this run (`HEX_APPLICATION_SUPPORT_DIR`). A normal
/// Windows executable keeps even its machine-local files beside itself.
fn local_support_dir() -> Option<PathBuf> {
    if !cfg!(windows) || std::env::var_os("HEX_APPLICATION_SUPPORT_DIR").is_some() {
        return None;
    }
    #[cfg(windows)]
    if this_data_root().is_some() {
        return None;
    }
    std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(|local| PathBuf::from(local).join("hex"))
}

/// A managed install's machine-local folder: the one beside the install,
/// adopting an older Hex's `%LOCALAPPDATA%\hex` copy once.
#[cfg(windows)]
fn adopt_machine_local(roaming: &Path, name: &str) -> PathBuf {
    let legacy = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(|local| PathBuf::from(local).join("hex").join(name));
    legacy.map_or_else(
        || roaming.to_path_buf(),
        |legacy| move_once(&legacy, roaming),
    )
}

/// No per-machine folder off the support folder elsewhere.
#[cfg(not(windows))]
fn adopt_machine_local(roaming: &Path, _name: &str) -> PathBuf {
    roaming.to_path_buf()
}

/// `local`, after moving `roaming` there if only `roaming` exists. A move
/// that fails leaves `roaming` in place and in use. Renaming works only
/// within a volume, so a move across volumes (the old `%LOCALAPPDATA%` copy
/// to an exe on another drive) copies through a `.partial` folder first and
/// deletes the source only once the copy is complete and renamed into place.
fn move_once(roaming: &Path, local: &Path) -> PathBuf {
    if local.exists() || !roaming.exists() {
        return local.to_path_buf();
    }
    let moved = local
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::rename(roaming, local))
        .or_else(|_| move_across(roaming, local));
    match moved {
        Ok(()) => local.to_path_buf(),
        Err(_) => roaming.to_path_buf(),
    }
}

/// Copies `roaming` into place at `local` through a `.partial` folder, then
/// deletes `roaming` — a rename's result, but working across volumes. The
/// `.partial` name, so a crash mid-copy never leaves a half-copied folder
/// that looks finished; a later run finds no `local` and copies again.
fn move_across(roaming: &Path, local: &Path) -> std::io::Result<()> {
    let partial = local.with_extension("partial");
    let _ = fs::remove_dir_all(&partial);
    copy_dir(roaming, &partial)
        .and_then(|()| fs::rename(&partial, local))
        .and_then(|()| fs::remove_dir_all(roaming))
}

/// Copies a whole folder, files and subfolders, preserving contents.
fn copy_dir(source: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Creates `directory` readable only by this user, and tightens it if an
/// older Hex left it open. Tightening is best-effort: a folder Hex can't
/// chmod (another owner, a mount that refuses it) is logged, never fatal.
/// Windows folders already inherit the per-user `%APPDATA%` permissions.
pub fn create_private_dir(directory: &Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)?;
    #[cfg(unix)]
    tighten(directory, 0o700);
    Ok(())
}

/// Opens `path` to append, readable only by this user, tightening a file an
/// older Hex created open (best-effort, as `create_private_dir`).
pub fn open_private_append(path: &Path) -> std::io::Result<fs::File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options.open(path)?;
    #[cfg(unix)]
    tighten(path, 0o600);
    Ok(file)
}

#[cfg(unix)]
fn tighten(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
        tracing::warn!(path = %path.display(), %error, "could not make this private");
    }
}

/// Creates the logs directory, appends process diagnostics to `process.log`
/// alongside stderr, installs the global tracing subscriber, and routes Ctrl-C
/// to `shutdown`. Returns the logs directory.
pub fn init_process_logging(shutdown: &'static AtomicBool) -> Result<PathBuf> {
    let log_dir = logs_dir()?;
    create_private_dir(&log_dir)?;
    let process_log = open_private_append(&log_dir.join("process.log"))?;
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("hex=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr.and(Mutex::new(process_log)))
        .init();
    log_panics();
    ctrlc::set_handler(|| shutdown.store(true, Ordering::Relaxed))?;
    Ok(log_dir)
}

/// Records every panic in `process.log` before the installed report runs.
/// Desktop apps have no console, so the default stderr report is otherwise
/// lost and a crashed worker thread leaves no trace.
fn log_panics() {
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(
            thread = std::thread::current().name().unwrap_or("unnamed"),
            backtrace = %std::backtrace::Backtrace::force_capture(),
            "panic: {info}"
        );
        report(info);
    }));
}

pub fn opencode_workspace() -> Result<PathBuf> {
    Ok(support_dir()?.join("opencode"))
}

pub fn local_api_discovery_file() -> Result<PathBuf> {
    Ok(support_dir()?.join("local-api.json"))
}

pub fn personal_commands_status_file() -> Result<PathBuf> {
    Ok(support_dir()?.join("personal-commands.json"))
}

/// The config hash the person allowed to use `fetch` and run programs. It
/// lives here, not in the config folder, so a config cannot allow itself.
pub fn transform_trust_file() -> Result<PathBuf> {
    Ok(support_dir()?.join("transform-trust.json"))
}

pub fn personal_commands_workspace() -> Result<PathBuf> {
    user_config_dir()
}

/// The per-user config folder holding `hex.config.ts` on every OS: `~/.config/hex`, or `~/.config/hex-test` for `Hex Test.app`.
fn user_config_dir() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| eyre!("home directory is unavailable"))?;
    Ok(user_config_dir_in(&home, is_macos_test_build()))
}

/// The home folder. On Windows `USERPROFILE` names it, as `HOME` does
/// elsewhere, so a copy of Hex launched with another profile folder (an
/// isolated benchmark) reads that folder's config; `dirs::home_dir` asks the
/// shell for the signed-in user's own folder instead. Every path Hex derives
/// from the user's folders goes through these, as Annotate's engine does
/// (`scrawl_router::user_dirs`).
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|profile| !profile.is_empty()) {
        return Some(profile.into());
    }
    dirs::home_dir()
}

/// The roaming data folder: `APPDATA` on Windows, else `dirs::data_dir`.
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(roaming) = std::env::var_os("APPDATA").filter(|value| !value.is_empty()) {
        return Some(roaming.into());
    }
    dirs::data_dir()
}

/// The Desktop folder (Labs › Screenshots saves there by default):
/// `dirs::desktop_dir`, else `Desktop` in the home folder.
pub fn desktop_dir() -> PathBuf {
    dirs::desktop_dir()
        .or_else(|| home_dir().map(|home| home.join("Desktop")))
        .unwrap_or_else(|| PathBuf::from("Desktop"))
}

/// The Documents folder: `dirs::document_dir`, or `Documents` in a Windows
/// profile `USERPROFILE` points to that is not the signed-in user's own.
pub fn document_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let (Some(profile), Some(own)) = (home_dir(), dirs::home_dir())
        && !profile
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(own.to_string_lossy().trim_end_matches(['\\', '/']))
    {
        return Some(profile.join("Documents"));
    }
    dirs::document_dir()
}

fn user_config_dir_in(home: &std::path::Path, test_build: bool) -> PathBuf {
    home.join(".config")
        .join(if test_build { "hex-test" } else { "hex" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_roaming_folder_moves_to_the_local_one_once() {
        let root = std::env::temp_dir().join(format!("hex-local-move-{}", std::process::id()));
        let roaming = root.join("Roaming").join("hex").join("models");
        let local = root.join("Local").join("hex").join("models");
        fs::create_dir_all(&roaming).unwrap();
        fs::write(roaming.join("model.gguf"), b"weights").unwrap();

        assert_eq!(move_once(&roaming, &local), local);
        assert_eq!(fs::read(local.join("model.gguf")).unwrap(), b"weights");
        assert!(!roaming.exists());

        // Later runs, and fresh installs, use the local folder as it is.
        assert_eq!(move_once(&roaming, &local), local);
        let fresh = root.join("Fresh").join("models");
        assert_eq!(move_once(&root.join("none"), &fresh), fresh);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn a_managed_install_moves_the_machine_local_folder_beside_it() {
        let root = std::env::temp_dir().join(format!("hex-adopt-{}", std::process::id()));
        let legacy = root.join("Local").join("hex").join("logs");
        let beside = root.join("Install").join("logs");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("process.log"), b"diagnostics").unwrap();

        // `adopt_machine_local` points `move_once` at the old
        // `%LOCALAPPDATA%\hex` folder, so the move is what carries the logs
        // over — and a failed move keeps the old copy in use.
        assert_eq!(move_once(&legacy, &beside), beside);
        assert_eq!(
            fs::read(beside.join("process.log")).unwrap(),
            b"diagnostics"
        );
        assert_eq!(move_once(&legacy, &beside), beside);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn only_a_versions_layout_names_the_managed_root() {
        let root = std::env::temp_dir().join(format!("hex-managed-{}", std::process::id()));
        let managed = root.join("Apps").join("Hex");
        fs::create_dir_all(managed.join(r"versions\1.0.0")).unwrap();
        let installed = managed.join(r"versions\1.0.0\hex.exe");
        assert_eq!(managed_root(&installed), Some(managed.clone()));
        assert_eq!(data_root(&installed), Some(managed));
        // A build keeps its data beside the exe, but never claims to be an
        // install the updater can replace.
        let built = root.join("checkout").join(r"target\debug\hex.exe");
        fs::create_dir_all(built.parent().unwrap()).unwrap();
        assert_eq!(managed_root(&built), None);
        assert_eq!(
            data_root(&built),
            Some(built.parent().unwrap().to_path_buf())
        );
        let loose = root.join("hex.exe");
        fs::write(&loose, b"").unwrap();
        assert_eq!(managed_root(&loose), None);
        assert_eq!(data_root(&loose), Some(root.clone()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_move_that_fails_keeps_the_roaming_folder_in_use() {
        let root = std::env::temp_dir().join(format!("hex-local-stay-{}", std::process::id()));
        let roaming = root.join("Roaming").join("models");
        fs::create_dir_all(&roaming).unwrap();
        // A file where the local parent folder should be makes the move fail.
        fs::write(root.join("Local"), b"").unwrap();

        assert_eq!(
            move_once(&roaming, &root.join("Local").join("models")),
            roaming
        );
        assert!(roaming.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_move_across_volumes_copies_through_a_partial_folder() {
        // Rename cannot move a folder between volumes, which the old
        // `%LOCALAPPDATA%` copy to an exe on another drive always is.
        let root = std::env::temp_dir().join(format!("hex-cross-{}", std::process::id()));
        let roaming = root.join("models");
        fs::create_dir_all(roaming.join("nested")).unwrap();
        fs::write(roaming.join("model.gguf"), b"weights").unwrap();
        fs::write(roaming.join("nested").join("kept.txt"), b"kept").unwrap();
        // Another volume when the machine has one; the fallback path itself
        // works the same within one.
        let drive = ["D:\\", "E:\\"]
            .into_iter()
            .find(|drive| {
                std::path::Path::new(drive).is_dir() && !std::env::temp_dir().starts_with(drive)
            })
            .map(PathBuf::from)
            .unwrap_or_else(|| root.clone());
        let dest = drive
            .join(format!("hex-xvol-{}", std::process::id()))
            .join("models");
        let _ = fs::remove_dir_all(dest.parent().unwrap());

        move_across(&roaming, &dest).unwrap();
        assert_eq!(fs::read(dest.join("model.gguf")).unwrap(), b"weights");
        assert_eq!(
            fs::read(dest.join("nested").join("kept.txt")).unwrap(),
            b"kept"
        );
        assert!(!roaming.exists(), "the source is deleted once copied");
        assert!(!dest.with_extension("partial").exists());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(dest.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_logs_folder_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("hex-private-dir-{}", std::process::id()));
        let logs = root.join("logs");
        fs::create_dir_all(&logs).unwrap();
        fs::set_permissions(&logs, fs::Permissions::from_mode(0o755)).unwrap();

        create_private_dir(&logs).unwrap();
        create_private_dir(&root.join("fresh")).unwrap();

        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&logs), 0o700);
        assert_eq!(mode(&root.join("fresh")), 0o700);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn each_build_has_its_own_config_folder() {
        let home = std::path::Path::new("/Users/kit");
        assert_eq!(
            user_config_dir_in(home, false),
            home.join(".config").join("hex")
        );
        assert_eq!(
            user_config_dir_in(home, true),
            home.join(".config").join("hex-test")
        );
    }

    #[cfg(windows)]
    #[test]
    fn relocating_data_moves_files_and_leaves_the_install_behind() {
        let root = std::env::temp_dir().join(format!("hex-relocate-{}", std::process::id()));
        let current = root.join("old");
        let new = root.join("new");
        fs::create_dir_all(current.join("models")).unwrap();
        fs::write(current.join("settings.json"), b"settings").unwrap();
        fs::write(current.join("models/model.gguf"), b"weights").unwrap();
        fs::write(current.join("install.json"), b"receipt").unwrap();
        fs::create_dir_all(current.join("versions/1.0.0")).unwrap();

        relocate_data(&current, &new).unwrap();
        assert_eq!(fs::read(new.join("settings.json")).unwrap(), b"settings");
        assert_eq!(fs::read(new.join("models/model.gguf")).unwrap(), b"weights");
        assert!(
            !new.join("install.json").exists(),
            "the install receipt stays with the executable"
        );
        assert!(!new.join("versions").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
