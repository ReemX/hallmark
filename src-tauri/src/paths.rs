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
        None => tracing::warn!(
            "discovery: Steam install NOT detected (HKLM and HKCU keys both absent)"
        ),
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

// ---------- Goldberg + local_save.txt + appmanifest helpers ----------
//
// These are stubs for Task 1 — Task 2 fills in the real bodies. They exist now so
// `discover()` and the type-shape tests compile.

fn goldberg_default_roots() -> Vec<PathBuf> {
    // Filled in Task 2.
    Vec::new()
}

fn scan_local_save_redirects(_libraries: &[PathBuf]) -> Vec<GoldbergRedirect> {
    // Filled in Task 2.
    Vec::new()
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
        let tmp = std::env::temp_dir().join(format!("hallmark-libfolders-{}", uuid::Uuid::new_v4()));
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
