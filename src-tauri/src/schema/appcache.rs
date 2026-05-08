//! Local Steam appcache reader. For Phase 2, we only resolve game-header
//! icons from `<steam_root>/appcache/librarycache/<app_id>_*.{jpg,png}` —
//! achievement icons are NOT stored here for legitimate Steam games
//! (per RESEARCH.md Open Question #4). Goldberg-emulated games surface
//! achievement icons via the Goldberg achievements.json `icon` field
//! (parsed in goldberg_meta.rs).
//!
//! This module uses pure file I/O — no Win32 APIs — so it has no
//! `#[cfg(target_os)]` arms.

use std::path::{Path, PathBuf};

/// Find a game-header icon for an app in the local Steam appcache.
/// Returns the first matching file path under `<steam_root>/appcache/librarycache/`.
/// Looks for `<app_id>_library_600x900.jpg`, `<app_id>_library_hero.jpg`,
/// `<app_id>_logo.png`, in that order.
///
/// Returns None if Steam isn't installed, the appcache is missing, or no
/// matching file exists. Plan 02 callers log + continue with the next leg
/// of the resolution chain.
pub fn find_local_icon(steam_root: &Path, app_id: u64) -> Option<PathBuf> {
    let cache = steam_root.join("appcache").join("librarycache");
    if !cache.exists() {
        tracing::debug!(path = %cache.display(), "appcache/librarycache not present");
        return None;
    }
    // Try preferred filenames first (largest/highest-quality), then any match.
    let candidates = [
        format!("{app_id}_library_600x900.jpg"),
        format!("{app_id}_library_hero.jpg"),
        format!("{app_id}_logo.png"),
        format!("{app_id}_header.jpg"),
    ];
    for name in candidates.iter() {
        let p = cache.join(name);
        if p.exists() {
            tracing::debug!(app_id, path = %p.display(), "appcache icon hit");
            return Some(p);
        }
    }
    // Last resort: any file starting with `<app_id>_` in the directory.
    if let Ok(entries) = std::fs::read_dir(&cache) {
        let prefix = format!("{app_id}_");
        for entry in entries.flatten() {
            let n = entry.file_name();
            let s = n.to_string_lossy();
            if s.starts_with(&prefix) {
                let p = entry.path();
                if p.is_file() {
                    tracing::debug!(app_id, path = %p.display(), "appcache icon hit (prefix)");
                    return Some(p);
                }
            }
        }
    }
    tracing::debug!(app_id, "no appcache icon found");
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fresh_tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "hallmark-appcache-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn returns_none_when_no_appcache_dir() {
        let root = fresh_tmp("no-cache");
        assert!(find_local_icon(&root, 480).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_preferred_filename() {
        let root = fresh_tmp("preferred");
        let cache = root.join("appcache").join("librarycache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("480_library_600x900.jpg"), b"fake").unwrap();
        fs::write(cache.join("480_logo.png"), b"fake").unwrap();
        let got = find_local_icon(&root, 480).unwrap();
        assert!(
            got.ends_with("480_library_600x900.jpg"),
            "got {}",
            got.display()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn falls_back_to_prefix_match() {
        let root = fresh_tmp("prefix");
        let cache = root.join("appcache").join("librarycache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("480_someotherfile.dat"), b"fake").unwrap();
        let got = find_local_icon(&root, 480).unwrap();
        assert!(got
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("480_"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn isolates_by_app_id() {
        let root = fresh_tmp("iso");
        let cache = root.join("appcache").join("librarycache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("999_logo.png"), b"fake").unwrap();
        assert!(
            find_local_icon(&root, 480).is_none(),
            "different app should miss"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
