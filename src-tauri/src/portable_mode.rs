//! Portable-vs-installed-mode detection + --silent argv parsing.
//! Phase 4 Plan 04-03 owns implementation. See CONTEXT.md D-08, D-23.

/// True if the running exe is NOT inside `%LOCALAPPDATA%\Hallmark`.
pub fn is_portable() -> bool {
    tracing::warn!("portable_mode::is_portable STUB — Plan 04-03 not yet implemented; defaulting to non-portable");
    false
}

/// True if the process was launched with `--silent` (HKCU\Run autostart).
pub fn is_silent_launch() -> bool {
    std::env::args().any(|a| a == "--silent")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_silent_launch_in_test_runner() {
        // cargo test does not pass --silent
        assert!(!is_silent_launch());
    }
}
