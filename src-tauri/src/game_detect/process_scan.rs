//! sysinfo-based process scanner. For each running process, attempt to map
//! its exe path to a Steam app_id via:
//!   1. <library>/steamapps/common/<installdir>/  → appmanifest_*.acf appid lookup
//!   2. Goldberg redirect root path  → appid from path segment
//!
//! Matches D-21 fallback path. The Steam-state path (D-21 authoritative leg)
//! is steam_state.rs — but in Phase 2 we don't have a public Steam IPC for
//! "currently playing app", so process scanning is the practical implementation
//! for both legs. See CONTEXT.md "## Phase 2 Implementation Notes" for the
//! D-21 Steam-state-authoritative-leg deferral to Phase 3.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessesToUpdate, System};

use crate::paths::appmanifest_lookup;

/// One running process that mapped to a Steam app_id.
#[derive(Debug, Clone, PartialEq)]
pub struct RunningGame {
    pub pid: u32,
    pub app_id: u64,
    pub name: String,        // process name
    pub exe_path: PathBuf,
}

/// Scan currently-running processes and return those whose exe paths map
/// to a known Steam app via appmanifest installdir matching OR a Goldberg
/// redirect root.
///
/// `sys` should be refreshed by the caller (refresh_processes) before calling.
/// `steam_libraries` is from Phase 1's path discovery (libraryfolders.vdf parsing).
/// `goldberg_redirect_roots` is the directory list from Phase 1's local_save.txt resolution.
pub fn scan_running_games(
    sys: &System,
    steam_libraries: &[PathBuf],
    goldberg_redirect_roots: &HashMap<PathBuf, u64>,
) -> Vec<RunningGame> {
    // Build per-library installdir → appid maps once per call.
    let library_maps: Vec<(PathBuf, HashMap<String, u64>)> = steam_libraries
        .iter()
        .map(|lib| (lib.clone(), appmanifest_lookup(lib)))
        .collect();

    let mut out = Vec::new();
    for (pid, proc) in sys.processes() {
        let Some(exe) = proc.exe() else { continue };
        let exe_path = exe.to_path_buf();
        let name = proc.name().to_string_lossy().to_string();

        // Leg 1: exe inside a steamapps/common/<installdir>?
        if let Some(app_id) = match_steam_library(&exe_path, &library_maps) {
            out.push(RunningGame { pid: pid.as_u32(), app_id, name: name.clone(), exe_path: exe_path.clone() });
            continue;
        }
        // Leg 2: exe inside a known Goldberg redirect root?
        if let Some(app_id) = match_goldberg_root(&exe_path, goldberg_redirect_roots) {
            out.push(RunningGame { pid: pid.as_u32(), app_id, name, exe_path });
        }
    }
    out
}

/// For each library, check if exe_path is under steamapps/common/<dir>; if so,
/// look up the dir in that library's installdir → appid map.
fn match_steam_library(
    exe_path: &Path,
    library_maps: &[(PathBuf, HashMap<String, u64>)],
) -> Option<u64> {
    for (lib, map) in library_maps {
        let common = lib.join("steamapps").join("common");
        let Ok(rel) = exe_path.strip_prefix(&common) else { continue };
        // First component of rel is the installdir name.
        let Some(first) = rel.components().next() else { continue };
        // BL-01: appmanifest_lookup lowercases keys; match on lowercase to be consistent.
        let installdir = first.as_os_str().to_string_lossy().to_ascii_lowercase();
        if let Some(&app_id) = map.get(&installdir) { return Some(app_id); }
    }
    None
}

/// Goldberg redirect roots are directories with a known appid (per Phase 1
/// local_save.txt scan). If exe_path is under any such root, use that appid.
fn match_goldberg_root(
    exe_path: &Path,
    goldberg_roots: &HashMap<PathBuf, u64>,
) -> Option<u64> {
    for (root, &app_id) in goldberg_roots {
        if exe_path.starts_with(root) { return Some(app_id); }
    }
    None
}

/// Helper: refresh sysinfo + return current snapshot of mapped games.
pub fn refresh_and_scan(
    sys: &mut System,
    steam_libraries: &[PathBuf],
    goldberg_redirect_roots: &HashMap<PathBuf, u64>,
) -> Vec<RunningGame> {
    // sysinfo 0.38 API: second arg is `remove_dead_processes: bool`.
    sys.refresh_processes(ProcessesToUpdate::All, true);
    scan_running_games(sys, steam_libraries, goldberg_redirect_roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fresh_tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("hallmark-procscan-{}-{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn match_goldberg_root_finds_exe_under_root() {
        let mut roots = HashMap::new();
        roots.insert(PathBuf::from("/games/MyGame"), 480_u64);
        assert_eq!(
            match_goldberg_root(&PathBuf::from("/games/MyGame/bin/MyGame.exe"), &roots),
            Some(480)
        );
        assert_eq!(
            match_goldberg_root(&PathBuf::from("/games/OtherGame/bin/OtherGame.exe"), &roots),
            None
        );
    }

    #[test]
    fn match_steam_library_uses_installdir() {
        let lib = PathBuf::from("/steam/library");
        let mut map = HashMap::new();
        // appmanifest_lookup lowercases keys (BL-01); match_steam_library lowercases
        // the path component too, so the lookup must use the same case.
        map.insert("mygame".to_string(), 480_u64);
        let library_maps = vec![(lib.clone(), map)];
        let exe = PathBuf::from("/steam/library/steamapps/common/MyGame/bin/MyGame.exe");
        assert_eq!(match_steam_library(&exe, &library_maps), Some(480));
        // Same library but different installdir not in map → None
        let exe2 = PathBuf::from("/steam/library/steamapps/common/Unmapped/bin/x.exe");
        assert_eq!(match_steam_library(&exe2, &library_maps), None);
        // Outside steamapps/common → None
        let exe3 = PathBuf::from("/steam/library/something/else.exe");
        assert_eq!(match_steam_library(&exe3, &library_maps), None);
    }

    #[test]
    fn match_steam_library_with_real_appmanifest() {
        // Build a real fixture: <tmp>/steamapps/common/MyGame/x.exe + <tmp>/steamapps/appmanifest_480.acf
        let lib = fresh_tmp("real-acf");
        let acf_dir = lib.join("steamapps");
        fs::create_dir_all(&acf_dir).unwrap();
        fs::create_dir_all(acf_dir.join("common").join("MyGame")).unwrap();
        fs::write(
            acf_dir.join("appmanifest_480.acf"),
            r#"
"AppState"
{
    "appid"      "480"
    "installdir" "MyGame"
}
"#,
        ).unwrap();
        let map = appmanifest_lookup(&lib);
        assert_eq!(map.get("mygame"), Some(&480_u64));
        let library_maps = vec![(lib.clone(), map)];
        let exe = lib.join("steamapps").join("common").join("MyGame").join("x.exe");
        // appmanifest_lookup lowercases keys (BL-01); match_steam_library must also lowercase
        assert_eq!(match_steam_library(&exe, &library_maps), Some(480));
        let _ = fs::remove_dir_all(&lib);
    }
}
