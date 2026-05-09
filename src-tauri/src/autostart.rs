//! HKCU\Run autostart helper.
//!
//! Manages the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark` registry
//! value that makes Hallmark start with Windows. Only ever touches HKCU — per-user
//! registry only (D-07 hard rule; see CONTEXT.md).
//!
//! The exe path is double-quoted in the value string so spaces in the install path
//! (e.g. `C:\Users\First Last\...`) are preserved as a single argv[0] token. The
//! `--silent` flag is appended so the companion window does not auto-open on startup
//! (D-08). The `format_run_value` helper is `pub(crate)` so the unit test for quoting
//! does not require a live HKCU write.

#[cfg(target_os = "windows")]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const VALUE_NAME: &str = "Hallmark";

/// Format the HKCU\Run value string. Quoted exe path + `--silent` arg.
/// `pub(crate)` for unit testing without touching the real registry.
#[cfg(target_os = "windows")]
pub(crate) fn format_run_value(exe: &std::path::Path) -> String {
    format!(r#""{}" --silent"#, exe.display())
}

/// Read live HKCU\Run state for the "Hallmark" value.
/// Returns `Ok(false)` when the value is absent (not an error).
#[cfg(target_os = "windows")]
pub fn is_enabled() -> anyhow::Result<bool> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
        Ok(key) => match key.get_value::<String, _>(VALUE_NAME) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Write `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark`.
/// Idempotent — calling twice leaves exactly one value.
/// The value format is `"<exe-path>" --silent` (path is double-quoted for argv safety).
#[cfg(target_os = "windows")]
pub fn enable() -> anyhow::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let exe = std::env::current_exe()?;
    let value = format_run_value(&exe);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _disp) = hkcu.create_subkey(RUN_KEY)?;
    key.set_value(VALUE_NAME, &value)?;
    tracing::info!(value = %value, "autostart enabled (HKCU\\...\\Run\\Hallmark)");
    Ok(())
}

/// Remove the "Hallmark" value from `HKCU\Run` (does NOT delete the key itself).
/// Idempotent — calling on absent value returns `Ok(())` without error.
/// Uses `KEY_SET_VALUE` (minimum-rights principle — sufficient for `delete_value`).
#[cfg(target_os = "windows")]
pub fn disable() -> anyhow::Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        Ok(key) => match key.delete_value(VALUE_NAME) {
            Ok(()) => {
                tracing::info!("autostart disabled (HKCU\\...\\Run\\Hallmark removed)");
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!("autostart::disable: value already absent");
                Ok(())
            }
            Err(e) => Err(e.into()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("autostart::disable: Run key absent");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

// ---- Non-Windows stubs (cross-platform compile friendliness) ----

/// Non-Windows stub — always returns `Ok(false)`.
#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> anyhow::Result<bool> {
    Ok(false)
}

/// Non-Windows stub — returns an error since autostart requires Windows registry.
#[cfg(not(target_os = "windows"))]
pub fn enable() -> anyhow::Result<()> {
    anyhow::bail!("autostart not supported on this OS")
}

/// Non-Windows stub — no-op.
#[cfg(not(target_os = "windows"))]
pub fn disable() -> anyhow::Result<()> {
    Ok(())
}

// ---- Tests ----

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Pure string-format test: no registry access.
    /// Verifies that exe paths with spaces are correctly double-quoted so that
    /// the Windows argv splitter treats the exe path as a single token (T-04-07).
    #[test]
    fn value_quoting_preserves_spaces_in_path() {
        let exe = PathBuf::from(r"C:\Users\First Last\AppData\Local\Hallmark\hallmark.exe");
        let v = format_run_value(&exe);
        assert_eq!(
            v,
            r#""C:\Users\First Last\AppData\Local\Hallmark\hallmark.exe" --silent"#
        );
    }

    /// Plain path without spaces also round-trips correctly.
    #[test]
    fn value_quoting_no_spaces() {
        let exe = PathBuf::from(r"C:\Hallmark\hallmark.exe");
        let v = format_run_value(&exe);
        assert_eq!(v, r#""C:\Hallmark\hallmark.exe" --silent"#);
    }

    /// Live HKCU round-trip — only run when explicitly invoked, since it touches
    /// the user's actual Run key. Use `--ignored` flag to opt in.
    #[test]
    #[ignore = "live HKCU write — run with `cargo test -- --ignored`"]
    fn enable_then_is_enabled_round_trip() {
        // NOTE: this test mutates the real HKCU — only run on a clean dev machine.
        let initial = is_enabled().unwrap();
        if initial {
            disable().unwrap();
            assert!(!is_enabled().unwrap());
        }
        enable().unwrap();
        assert!(is_enabled().unwrap(), "after enable, is_enabled returns true");
        // Idempotent double-enable
        enable().unwrap();
        assert!(is_enabled().unwrap(), "double-enable still enabled");
        disable().unwrap();
        assert!(!is_enabled().unwrap(), "after disable, is_enabled returns false");
        // Idempotent double-disable
        disable().unwrap();
    }
}
