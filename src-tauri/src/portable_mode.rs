//! Portable-vs-installed mode detection + --silent argv parsing.
//! D-23: when running OUTSIDE `%LOCALAPPDATA%\Hallmark`, treat as portable
//! (extracted .zip, dev build, USB drive). Disables the auto-updater check.

use std::path::Path;

/// True if the running exe is NOT inside `%LOCALAPPDATA%\Hallmark`.
/// Failure-to-detect (current_exe error, no LOCALAPPDATA) defaults to false
/// (safest — treat as installed; updater enabled).
pub fn is_portable() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "current_exe failed; assuming installed mode");
            return false;
        }
    };
    let exe_parent = match exe.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            tracing::warn!("exe has no parent directory; assuming installed mode");
            return false;
        }
    };
    let installed = match dirs::data_local_dir() {
        Some(p) => p.join("Hallmark"),
        None => {
            tracing::warn!("data_local_dir unavailable; assuming installed mode");
            return false;
        }
    };
    let portable = is_portable_with(&exe_parent, &installed);
    tracing::info!(
        portable,
        exe_parent = %exe_parent.display(),
        installed = %installed.display(),
        "portable_mode resolved"
    );
    portable
}

/// True if the process was launched with `--silent` (HKCU\Run autostart).
/// One-liner — no CLI plugin needed.
pub fn is_silent_launch() -> bool {
    std::env::args().any(|a| a == "--silent")
}

/// Pure helper for testing without touching the real environment.
/// Compares canonical forms; falls back to false if either path cannot be
/// canonicalized (e.g., installed dir does not exist because we're running
/// from a fresh extracted .zip — that case returns false, the safest default,
/// meaning "assume installed so updater runs").
pub(crate) fn is_portable_with(exe_parent: &Path, installed: &Path) -> bool {
    let canon_exe = exe_parent.canonicalize().ok();
    let canon_inst = installed.canonicalize().ok();
    match (canon_exe, canon_inst) {
        (Some(a), Some(b)) => a != b,
        // Either path failed to canonicalize. Most-common reason: installed dir
        // doesn't exist (portable/dev scenario). However, defaulting to false
        // (assumed installed) is the safer choice — the updater check runs but
        // fails gracefully, rather than silently disabling updates on real installs.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn is_portable_with_returns_false_on_equal_paths() {
        // Use the test runner's working directory as a stable existing path.
        let cwd = env::current_dir().unwrap();
        assert!(
            !is_portable_with(&cwd, &cwd),
            "equal paths => not portable (installed)"
        );
    }

    #[test]
    fn is_portable_with_returns_true_on_distinct_existing_paths() {
        let cwd = env::current_dir().unwrap();
        let temp = std::env::temp_dir();
        // Both paths exist on disk and are distinct — canonicalize succeeds for both.
        // Skip test if cwd happens to equal temp (very unlikely in CI).
        if cwd == temp {
            return;
        }
        assert!(
            is_portable_with(&cwd, &temp),
            "distinct existing paths => portable"
        );
    }

    #[test]
    fn is_portable_with_returns_false_when_installed_dir_missing() {
        let cwd = env::current_dir().unwrap();
        // Use a name that won't exist on any test machine.
        let phantom =
            std::env::temp_dir().join("HallmarkInstalledDirThatDoesNotExist_PortableTest_9f2a");
        // phantom doesn't exist — canonicalize fails => defaults to false (safest).
        assert!(
            !is_portable_with(&cwd, &phantom),
            "missing installed dir => not portable (safest default)"
        );
    }

    #[test]
    fn is_silent_launch_returns_false_in_test_runner() {
        // The cargo test invocation does not pass --silent.
        assert!(!is_silent_launch());
    }
}
