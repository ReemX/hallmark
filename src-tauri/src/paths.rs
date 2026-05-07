//! Path discovery for Steam libraries and Goldberg save roots (REQ DETECT-08).
//!
//! At startup, `discover()` reads the Windows registry to find Steam, parses both
//! known locations of `libraryfolders.vdf`, enumerates Goldberg's standard save
//! roots, and walks each Steam library's `steamapps/common/` to find every
//! `steam_api*.dll` and resolve its `local_save.txt` redirect (if any). Each
//! resolved redirect is paired with the appid resolved from the corresponding
//! `appmanifest_*.acf` in the same library so Plan 04's adapter can identify
//! the game even when the redirect target's directory is named "Save" or similar.
//!
//! Every discovered path is logged via `tracing::info!`, satisfying ROADMAP Phase 1
//! Success Criterion #5 ("all discovered paths logged at startup").
//!
//! # Why pure-ish functions over a service object
//!
//! Discovery happens once at startup. There's no per-event state to keep, so a
//! single `pub fn discover() -> DiscoveredPaths` is the right shape. Phase 4's
//! first-run wizard will call this same function from a UI button and surface the
//! `DiscoveredPaths` struct directly to the user.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One Goldberg `local_save.txt` redirect with its resolved appid.
/// The appid is resolved by walking the `steam_api*.dll`'s game directory back
/// to the matching `<library>\steamapps\appmanifest_<appid>.acf` and reading
/// the `installdir` field. Without this pairing, Plan 04's GoldbergAdapter
/// cannot identify the appid for a redirect whose target directory is not
/// numeric (e.g. `D:\Game1\Save\achievements.json` — parent is "Save").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldbergRedirect {
    pub target_path: PathBuf,
    pub app_id: u64,
}

/// Result of one-shot path discovery. Plan 04 consumes
/// `goldberg_save_roots` and `goldberg_local_save_redirects` to construct the
/// Goldberg adapter; Phase 3 will consume `steam_libraries` to find Steam's
/// per-user `userdata/<steamid>/<appid>/` paths.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredPaths {
    /// Steam install root (e.g. `D:\SteamLibrary\Steam`). `None` when Steam not detected.
    pub steam_install: Option<PathBuf>,
    /// All Steam library roots — each contains a `steamapps/` subdir. Always includes
    /// `steam_install` itself if present. Empty if Steam not detected.
    pub steam_libraries: Vec<PathBuf>,
    /// Goldberg default save roots that EXIST on this machine.
    pub goldberg_save_roots: Vec<PathBuf>,
    /// Resolved `local_save.txt` redirect targets paired with their appids.
    pub goldberg_local_save_redirects: Vec<GoldbergRedirect>,
}

/// Top-level discovery. Reads registry, parses VDFs, walks game install dirs.
/// Logs every discovered path. Pure side-effect-wise (no writes).
pub fn discover() -> DiscoveredPaths {
    let steam_install = read_steam_install();
    let steam_libraries = steam_install
        .as_ref()
        .map(|p| parse_libraryfolders(p))
        .unwrap_or_default();

    let goldberg_save_roots = goldberg_default_roots();
    let goldberg_local_save_redirects = scan_local_save_redirects(&steam_libraries);

    let result = DiscoveredPaths {
        steam_install: steam_install.clone(),
        steam_libraries: steam_libraries.clone(),
        goldberg_save_roots: goldberg_save_roots.clone(),
        goldberg_local_save_redirects: goldberg_local_save_redirects.clone(),
    };

    // Success Criterion #5: log every discovered path at startup.
    log_discovery(&result);
    result
}

/// Convenience: all paths the Goldberg adapter (Plan 04) wants to watch — defaults + redirects.
pub fn goldberg_watch_paths(d: &DiscoveredPaths) -> Vec<PathBuf> {
    let mut v = d.goldberg_save_roots.clone();
    v.extend(
        d.goldberg_local_save_redirects
            .iter()
            .map(|r| r.target_path.clone()),
    );
    v
}

/// Build the redirect→appid map Plan 04's GoldbergAdapter::new consumes.
/// Keys are redirect target paths; values are the resolved appids.
pub fn goldberg_redirect_map(d: &DiscoveredPaths) -> HashMap<PathBuf, u64> {
    d.goldberg_local_save_redirects
        .iter()
        .map(|r| (r.target_path.clone(), r.app_id))
        .collect()
}

fn log_discovery(d: &DiscoveredPaths) {
    match &d.steam_install {
        Some(p) => tracing::info!(path = %p.display(), "discovery: Steam install"),
        None => {
            tracing::warn!("discovery: Steam install NOT detected (HKLM and HKCU keys both absent)")
        }
    }
    for p in &d.steam_libraries {
        tracing::info!(path = %p.display(), "discovery: Steam library");
    }
    for p in &d.goldberg_save_roots {
        tracing::info!(path = %p.display(), "discovery: Goldberg save root (default)");
    }
    for r in &d.goldberg_local_save_redirects {
        tracing::info!(
            path = %r.target_path.display(),
            app_id = r.app_id,
            "discovery: Goldberg local_save.txt redirect"
        );
    }
    if d.goldberg_save_roots.is_empty() && d.goldberg_local_save_redirects.is_empty() {
        tracing::warn!(
            "discovery: NO Goldberg save paths found — Phase 1 watcher will have nothing to watch"
        );
    }
}

// ---------- Steam install registry ----------

/// Read the Steam install path from the Windows registry. Tries 64-bit user
/// (`HKLM\SOFTWARE\WOW6432Node\Valve\Steam\InstallPath`) first, then current-user
/// fallback (`HKCU\Software\Valve\Steam\SteamPath`).
#[cfg(target_os = "windows")]
fn read_steam_install() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("InstallPath") {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
            tracing::warn!(path = %path.display(), "Steam HKLM InstallPath does not exist on disk");
        }
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\Valve\Steam") {
        if let Ok(p) = key.get_value::<String, _>("SteamPath") {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
            tracing::warn!(path = %path.display(), "Steam HKCU SteamPath does not exist on disk");
        }
    }
    None
}

// Stub for non-Windows — Phase 1 is Windows-only but the cfg keeps the rest of the
// file compilable for hypothetical CI on Linux. (Per CLAUDE.md, Phase 1 is Win-only;
// this is just defensive scaffolding so `cargo check` stays green if anyone tries
// a Linux build.)
#[cfg(not(target_os = "windows"))]
fn read_steam_install() -> Option<PathBuf> {
    None
}

// ---------- libraryfolders.vdf parser ----------

/// Parse Steam's libraryfolders.vdf at one of the two known locations and return
/// every library root path it lists (plus the Steam install root, which is always
/// implicitly a library even if not listed).
pub(crate) fn parse_libraryfolders(steam_install: &Path) -> Vec<PathBuf> {
    let candidates = [
        steam_install.join("config").join("libraryfolders.vdf"), // post-2022 master
        steam_install.join("steamapps").join("libraryfolders.vdf"), // legacy / replicated
    ];
    for path in &candidates {
        if !path.exists() {
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "libraryfolders.vdf read failed; trying next candidate"
                );
                continue;
            }
        };
        let mut libs = parse_libraryfolders_text(&text);
        // Steam install itself is always a library (even when not listed).
        if !libs.iter().any(|l| l == steam_install) {
            libs.insert(0, steam_install.to_path_buf());
        }
        return libs;
    }
    // No VDF found at either location; fall back to just the install root.
    tracing::warn!(
        steam = %steam_install.display(),
        "no libraryfolders.vdf found at config\\ or steamapps\\"
    );
    vec![steam_install.to_path_buf()]
}

/// Parse the TEXT of a libraryfolders.vdf file (either post-2022 nested or legacy
/// flat format). Public-in-crate so tests can call it without writing fixture files
/// to disk.
pub(crate) fn parse_libraryfolders_text(text: &str) -> Vec<PathBuf> {
    use keyvalues_parser::Vdf;

    let vdf = match Vdf::parse(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "libraryfolders.vdf parse failed");
            return Vec::new();
        }
    };

    // The top-level key is "libraryfolders" (post-2022) or "LibraryFolders" (legacy);
    // case-insensitive match.
    if !vdf.key.eq_ignore_ascii_case("libraryfolders") {
        tracing::warn!(top_key = %vdf.key, "libraryfolders.vdf has unexpected top-level key");
        return Vec::new();
    }

    let Some(obj) = vdf.value.get_obj() else {
        tracing::warn!("libraryfolders.vdf top-level value is not an object");
        return Vec::new();
    };

    let mut libs = Vec::new();
    // Each entry under the top obj is keyed by a numeric string ("0", "1", ...);
    // value is either a sub-object with a "path" key (post-2022) or a string (legacy).
    for (entry_key, values) in obj.iter() {
        // Skip non-numeric keys (legacy format includes "TimeNextStatsReport", etc.).
        if entry_key.parse::<u32>().is_err() {
            continue;
        }

        for value in values.iter() {
            if let Some(s) = value.get_str() {
                // Legacy flat: the value IS the path string.
                libs.push(PathBuf::from(s));
            } else if let Some(sub_obj) = value.get_obj() {
                // Post-2022 nested: look for a "path" key inside the sub-object.
                if let Some(path_values) = sub_obj.get("path") {
                    if let Some(path_value) = path_values.first() {
                        if let Some(path_str) = path_value.get_str() {
                            libs.push(PathBuf::from(path_str));
                        }
                    }
                }
            }
        }
    }
    libs
}

// ---------- Goldberg default save roots ----------

/// Goldberg-emulated games write achievement state into one of three default roots.
/// Returns only those that exist on this machine.
fn goldberg_default_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // 1. %APPDATA%\Goldberg SteamEmu Saves\  (legacy default)
    // 2. %APPDATA%\GSE Saves\               (gbe_fork default; majority of 2024+ scene releases)
    if let Some(appdata) = dirs::data_dir() {
        for sub in ["Goldberg SteamEmu Saves", "GSE Saves"] {
            let p = appdata.join(sub);
            if p.exists() {
                roots.push(p);
            }
        }
    }

    // 3. %PUBLIC%\Documents\Goldberg SteamEmu Saves\  (rare; older releases)
    if let Some(public) = std::env::var_os("PUBLIC") {
        let p = PathBuf::from(public)
            .join("Documents")
            .join("Goldberg SteamEmu Saves");
        if p.exists() {
            roots.push(p);
        }
    }

    roots
}

// ---------- appmanifest lookup ----------

/// Build a `installdir → appid` map for one Steam library by scanning every
/// `<library>/steamapps/appmanifest_*.acf` file. Used by `scan_local_save_redirects`
/// to resolve the appid for each discovered redirect.
pub(crate) fn appmanifest_lookup(library: &Path) -> HashMap<String, u64> {
    use keyvalues_parser::Vdf;

    let mut map = HashMap::new();
    let steamapps = library.join("steamapps");
    if !steamapps.exists() {
        return map;
    }

    let entries = match std::fs::read_dir(&steamapps) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                library = %library.display(),
                error = %e,
                "appmanifest_lookup: failed to read steamapps directory"
            );
            return map;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Names look like `appmanifest_480.acf`.
        if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
            continue;
        }

        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "appmanifest read failed; skip"
                );
                continue;
            }
        };
        let vdf = match Vdf::parse(&text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "appmanifest parse failed; skip"
                );
                continue;
            }
        };
        // Top-level key is "AppState".
        if !vdf.key.eq_ignore_ascii_case("appstate") {
            continue;
        }
        let Some(obj) = vdf.value.get_obj() else {
            continue;
        };

        let appid = obj
            .get("appid")
            .and_then(|vs| vs.first())
            .and_then(|v| v.get_str())
            .and_then(|s| s.parse::<u64>().ok());
        let installdir = obj
            .get("installdir")
            .and_then(|vs| vs.first())
            .and_then(|v| v.get_str())
            .map(|s| s.to_string());

        if let (Some(id), Some(dir)) = (appid, installdir) {
            map.insert(dir, id);
        }
    }
    map
}

/// Walk the path UPWARD from the DLL directory until we hit
/// `<library>\steamapps\common\<installdir>`, then return that `installdir` segment.
/// Returns None if no `steamapps\common` ancestor is found.
fn extract_installdir_from_dll_path(dll_dir: &Path) -> Option<String> {
    // The DLL lives somewhere under `<library>\steamapps\common\<installdir>\...`.
    // We need to find the segment immediately after `common`.
    let components: Vec<&std::ffi::OsStr> = dll_dir.iter().collect();
    for i in 0..components.len().saturating_sub(2) {
        // Look for `steamapps` followed by `common`
        if components[i].eq_ignore_ascii_case("steamapps")
            && components[i + 1].eq_ignore_ascii_case("common")
            && i + 2 < components.len()
        {
            return components[i + 2].to_str().map(|s| s.to_string());
        }
    }
    None
}

// ---------- local_save.txt resolution ----------

/// Walk every Steam library's `steamapps\common\` looking for `steam_api*.dll`. For each
/// hit, check for a sibling `local_save.txt`; if present, resolve its content as a path
/// (absolute → use as-is; relative → join to DLL dir) and pair the result with the appid
/// resolved from `<library>\steamapps\appmanifest_*.acf` (matched by `installdir`).
/// Redirects with no matching appmanifest are dropped with a warn-level trace.
fn scan_local_save_redirects(libraries: &[PathBuf]) -> Vec<GoldbergRedirect> {
    let mut redirects = Vec::new();
    for lib in libraries {
        let common = lib.join("steamapps").join("common");
        if !common.exists() {
            continue;
        }
        let manifest_map = appmanifest_lookup(lib);

        // max_depth(8) is generous — most installs are at depth 2-4 (game/bin/steam_api64.dll).
        for entry in walkdir::WalkDir::new(&common).max_depth(8) {
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let name_lower = entry.file_name().to_string_lossy().to_lowercase();
            if name_lower != "steam_api.dll" && name_lower != "steam_api64.dll" {
                continue;
            }
            let dir = match entry.path().parent() {
                Some(d) => d,
                None => continue,
            };
            let local_save = dir.join("local_save.txt");
            if !local_save.exists() {
                continue;
            }
            let raw = match std::fs::read_to_string(&local_save) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        path = %local_save.display(),
                        error = %e,
                        "local_save.txt read failed; skipping"
                    );
                    continue;
                }
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let resolved = if Path::new(trimmed).is_absolute() {
                PathBuf::from(trimmed)
            } else {
                dir.join(trimmed)
            };
            if !resolved.exists() {
                tracing::warn!(
                    dll_dir = %dir.display(),
                    unresolved = %resolved.display(),
                    "local_save.txt points to non-existent path; skipping"
                );
                continue;
            }

            // Resolve appid from appmanifest matching the DLL's installdir.
            let installdir = match extract_installdir_from_dll_path(dir) {
                Some(s) => s,
                None => {
                    tracing::warn!(
                        dll_dir = %dir.display(),
                        "could not extract installdir from DLL path; skipping redirect"
                    );
                    continue;
                }
            };
            let app_id = match manifest_map.get(&installdir).copied() {
                Some(id) => id,
                None => {
                    tracing::warn!(
                        dll_dir = %dir.display(),
                        installdir = %installdir,
                        "no appmanifest_*.acf matches installdir; cannot resolve appid; skipping redirect"
                    );
                    continue;
                }
            };

            tracing::info!(
                dll_dir = %dir.display(),
                local_save_content = %trimmed,
                resolved = %resolved.display(),
                app_id = app_id,
                "Goldberg local_save.txt redirect resolved"
            );
            redirects.push(GoldbergRedirect {
                target_path: resolved,
                app_id,
            });
        }
    }
    redirects
}

#[cfg(test)]
mod tests_steam {
    use super::*;

    const POST_2022_VDF: &str = r#"
    "libraryfolders"
    {
        "0"
        {
            "path"      "C:\\Program Files (x86)\\Steam"
            "label"     ""
            "totalsize" "0"
        }
        "1"
        {
            "path"      "D:\\SteamLibrary"
            "label"     "Games"
            "totalsize" "1234567890"
        }
    }
    "#;

    const LEGACY_VDF: &str = r#"
    "LibraryFolders"
    {
        "TimeNextStatsReport"  "1234567890"
        "ContentStatsID"       "1234"
        "1"  "D:\\SteamLibrary"
        "2"  "E:\\AnotherLibrary"
    }
    "#;

    #[test]
    fn parse_libraryfolders_post_2022_nested() {
        let libs = parse_libraryfolders_text(POST_2022_VDF);
        assert_eq!(libs.len(), 2, "expected 2 libraries from post-2022 fixture");
        assert!(libs
            .iter()
            .any(|p| p == &PathBuf::from(r"C:\Program Files (x86)\Steam")));
        assert!(libs.iter().any(|p| p == &PathBuf::from(r"D:\SteamLibrary")));
    }

    #[test]
    fn parse_libraryfolders_legacy_flat() {
        let libs = parse_libraryfolders_text(LEGACY_VDF);
        assert_eq!(
            libs.len(),
            2,
            "expected 2 libraries from legacy fixture (timestamp keys filtered)"
        );
        assert!(libs.iter().any(|p| p == &PathBuf::from(r"D:\SteamLibrary")));
        assert!(libs
            .iter()
            .any(|p| p == &PathBuf::from(r"E:\AnotherLibrary")));
    }

    #[test]
    fn parse_libraryfolders_handles_escapes() {
        let libs = parse_libraryfolders_text(POST_2022_VDF);
        for p in &libs {
            let s = p.to_string_lossy();
            assert!(
                !s.contains(r"\\"),
                "double-backslashes should be unescaped: got {}",
                s
            );
        }
    }

    #[test]
    fn parse_libraryfolders_empty_text_returns_empty() {
        assert!(parse_libraryfolders_text("").is_empty());
        assert!(parse_libraryfolders_text("not even close to vdf").is_empty());
    }

    #[test]
    fn parse_libraryfolders_wraps_text_in_outer_disk_paths() {
        // Direct on-disk wrapper: write fixture VDF to a temp dir + call parse_libraryfolders.
        let tmp =
            std::env::temp_dir().join(format!("hallmark-libfolders-{}", uuid::Uuid::new_v4()));
        let config_dir = tmp.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("libraryfolders.vdf"), POST_2022_VDF).unwrap();

        let libs = parse_libraryfolders(&tmp);
        // Should include the steam-install root + both fixture entries.
        assert!(
            libs.contains(&tmp.to_path_buf()),
            "steam install root should be implicitly included"
        );
        assert!(
            libs.iter().any(|p| p == &PathBuf::from(r"D:\SteamLibrary")),
            "post-2022 entry should be parsed"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

#[cfg(test)]
mod tests_goldberg {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::Context as LayerContext;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Layer;

    fn fresh_tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-{}-{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Write a Steam-shaped `appmanifest_<appid>.acf` into `<library>\steamapps\`.
    fn write_appmanifest(library: &Path, app_id: u64, installdir: &str) {
        let steamapps = library.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();
        let content = format!(
            "\"AppState\"\n{{\n  \"appid\"      \"{}\"\n  \"name\"       \"Test Game\"\n  \"installdir\" \"{}\"\n}}\n",
            app_id, installdir
        );
        fs::write(
            steamapps.join(format!("appmanifest_{}.acf", app_id)),
            content,
        )
        .unwrap();
    }

    #[test]
    fn goldberg_default_roots_returns_only_existing() {
        // We cannot mock dirs::data_dir() without DI, so just assert the contract:
        // every returned path EXISTS on this machine. (The function may return 0 to 3.)
        for p in goldberg_default_roots() {
            assert!(
                p.exists(),
                "function should only return existing paths; got {:?}",
                p
            );
        }
    }

    #[test]
    fn appmanifest_lookup_returns_appid_for_installdir() {
        let lib = fresh_tmp("appmanifest");
        write_appmanifest(&lib, 480, "Spacewar");
        write_appmanifest(&lib, 730, "Counter-Strike Global Offensive");

        let map = appmanifest_lookup(&lib);
        assert_eq!(map.get("Spacewar").copied(), Some(480));
        assert_eq!(
            map.get("Counter-Strike Global Offensive").copied(),
            Some(730)
        );
        assert_eq!(map.get("UnknownGame").copied(), None);

        let _ = fs::remove_dir_all(&lib);
    }

    #[test]
    fn local_save_absolute_path_passes_through() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("FooGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        // Create a placeholder DLL beside which local_save.txt sits.
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();
        write_appmanifest(&lib, 12345, "FooGame");

        let target = fresh_tmp("absolute_save");
        let target_str = target.to_string_lossy().replace('/', "\\");
        fs::write(game_bin.join("local_save.txt"), &target_str).unwrap();

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        assert_eq!(
            redirects.len(),
            1,
            "expected exactly one redirect; got {:?}",
            redirects
        );
        assert_eq!(redirects[0].target_path, target);
        assert_eq!(redirects[0].app_id, 12345);

        let _ = fs::remove_dir_all(&lib);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn local_save_relative_path_resolves_against_dll_dir() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("BarGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api.dll"), b"placeholder").unwrap();
        write_appmanifest(&lib, 67890, "BarGame");
        // Relative target — must exist for the resolution to register.
        fs::create_dir_all(game_bin.join("save_data")).unwrap();
        fs::write(game_bin.join("local_save.txt"), ".\\save_data").unwrap();

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        let expected = game_bin.join("save_data");
        assert_eq!(redirects.len(), 1);
        assert_eq!(redirects[0].target_path, expected);
        assert_eq!(redirects[0].app_id, 67890);

        let _ = fs::remove_dir_all(&lib);
    }

    #[test]
    fn local_save_missing_target_is_filtered_out() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("BazGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();
        write_appmanifest(&lib, 11111, "BazGame");
        // local_save.txt points to a path that does NOT exist
        fs::write(game_bin.join("local_save.txt"), "Z:\\does\\not\\exist").unwrap();

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        assert!(
            redirects.is_empty(),
            "redirects should be empty when target is missing; got {:?}",
            redirects
        );

        let _ = fs::remove_dir_all(&lib);
    }

    #[test]
    fn local_save_no_local_save_txt_skipped() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("QuxGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();
        // Deliberately no local_save.txt

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        assert!(redirects.is_empty());

        let _ = fs::remove_dir_all(&lib);
    }

    #[test]
    fn local_save_trims_trailing_whitespace() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("WidgetGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();
        write_appmanifest(&lib, 22222, "WidgetGame");
        let target = fresh_tmp("trim_save");
        let target_str = target.to_string_lossy().replace('/', "\\");
        // Write with CRLF + trailing space
        fs::write(
            game_bin.join("local_save.txt"),
            format!("{}  \r\n", target_str),
        )
        .unwrap();

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        assert_eq!(redirects.len(), 1);
        assert_eq!(redirects[0].target_path, target);
        assert_eq!(redirects[0].app_id, 22222);

        let _ = fs::remove_dir_all(&lib);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn local_save_no_matching_appmanifest_is_skipped() {
        let lib = fresh_tmp("lib");
        let common = lib.join("steamapps").join("common");
        let game_bin = common.join("OrphanGame").join("bin");
        fs::create_dir_all(&game_bin).unwrap();
        fs::write(game_bin.join("steam_api64.dll"), b"placeholder").unwrap();
        // appmanifest exists but with a DIFFERENT installdir
        write_appmanifest(&lib, 33333, "SomeOtherGame");
        let target = fresh_tmp("orphan_save");
        let target_str = target.to_string_lossy().replace('/', "\\");
        fs::write(game_bin.join("local_save.txt"), &target_str).unwrap();

        let redirects = scan_local_save_redirects(&[lib.clone()]);
        assert!(
            redirects.is_empty(),
            "redirect should be skipped when no matching appmanifest exists; got {:?}",
            redirects
        );

        let _ = fs::remove_dir_all(&lib);
        let _ = fs::remove_dir_all(&target);
    }

    #[test]
    fn goldberg_watch_paths_combines_roots_and_redirects() {
        let d = DiscoveredPaths {
            steam_install: None,
            steam_libraries: vec![],
            goldberg_save_roots: vec![PathBuf::from(r"C:\Goldberg")],
            goldberg_local_save_redirects: vec![GoldbergRedirect {
                target_path: PathBuf::from(r"D:\Redirect"),
                app_id: 12345,
            }],
        };
        let watch = goldberg_watch_paths(&d);
        assert_eq!(watch.len(), 2);
        assert!(watch.contains(&PathBuf::from(r"C:\Goldberg")));
        assert!(watch.contains(&PathBuf::from(r"D:\Redirect")));
    }

    #[test]
    fn goldberg_redirect_map_keys_on_target_path() {
        let d = DiscoveredPaths {
            steam_install: None,
            steam_libraries: vec![],
            goldberg_save_roots: vec![],
            goldberg_local_save_redirects: vec![
                GoldbergRedirect {
                    target_path: PathBuf::from(r"D:\R1"),
                    app_id: 100,
                },
                GoldbergRedirect {
                    target_path: PathBuf::from(r"D:\R2"),
                    app_id: 200,
                },
            ],
        };
        let map = goldberg_redirect_map(&d);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&PathBuf::from(r"D:\R1")).copied(), Some(100));
        assert_eq!(map.get(&PathBuf::from(r"D:\R2")).copied(), Some(200));
    }

    // ---- Tracing capture (W-05 / Success Criterion #5) ----
    //
    // Capture tracing events into a Mutex<Vec<String>> so tests can assert that
    // log_discovery emits an info event for every populated category.

    struct VecLayer {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl<S: Subscriber> Layer<S> for VecLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
            use tracing::field::{Field, Visit};
            struct StringVisitor(String);
            impl Visit for StringVisitor {
                fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                    self.0.push_str(&format!(" {}={:?}", field.name(), value));
                }
                fn record_str(&mut self, field: &Field, value: &str) {
                    self.0.push_str(&format!(" {}={}", field.name(), value));
                }
            }
            let mut visitor = StringVisitor(String::new());
            event.record(&mut visitor);
            let line = format!("{} :: {}", event.metadata().level(), visitor.0);
            self.events.lock().unwrap().push(line);
        }
    }

    #[test]
    fn tracing_capture_records_info_event_for_each_discovery_category() {
        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = VecLayer {
            events: events.clone(),
        };

        // Run log_discovery inside a scoped subscriber so other tests are unaffected.
        let subscriber = tracing_subscriber::registry().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        let d = DiscoveredPaths {
            steam_install: Some(PathBuf::from(r"C:\FakeSteam")),
            steam_libraries: vec![
                PathBuf::from(r"C:\FakeSteam"),
                PathBuf::from(r"D:\FakeLibrary"),
            ],
            goldberg_save_roots: vec![PathBuf::from(r"C:\Goldberg")],
            goldberg_local_save_redirects: vec![GoldbergRedirect {
                target_path: PathBuf::from(r"D:\Redirect"),
                app_id: 12345,
            }],
        };
        log_discovery(&d);

        let captured = events.lock().unwrap().clone();
        assert!(
            captured.iter().any(|e| e.contains("Steam install")),
            "expected 'Steam install' info event; got: {:?}",
            captured
        );
        assert!(
            captured.iter().any(|e| e.contains("Steam library")),
            "expected 'Steam library' info event; got: {:?}",
            captured
        );
        assert!(
            captured.iter().any(|e| e.contains("Goldberg save root")),
            "expected 'Goldberg save root' info event; got: {:?}",
            captured
        );
        assert!(
            captured
                .iter()
                .any(|e| e.contains("local_save.txt redirect")),
            "expected 'local_save.txt redirect' info event; got: {:?}",
            captured
        );
        // Also confirm INFO level prevailed (no WARN for the populated categories).
        let info_count = captured.iter().filter(|e| e.starts_with("INFO")).count();
        assert!(
            info_count >= 4,
            "expected at least 4 INFO-level events for the four categories; got: {:?}",
            captured
        );
    }
}
