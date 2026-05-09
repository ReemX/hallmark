//! HKCU\Run autostart helper — Phase 4 Plan 04-02 owns implementation.
//! See CONTEXT.md D-07 (HKCU only, never HKLM), D-08 (--silent flag).

/// Read live HKCU\Run state for the "Hallmark" value.
pub fn is_enabled() -> anyhow::Result<bool> {
    tracing::warn!("autostart::is_enabled STUB — Plan 04-02 not yet implemented");
    Ok(false)
}

/// Write `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Hallmark`.
pub fn enable() -> anyhow::Result<()> {
    tracing::warn!("autostart::enable STUB — Plan 04-02 not yet implemented");
    Ok(())
}

/// Remove the "Hallmark" value from HKCU\Run (does NOT delete the key itself).
pub fn disable() -> anyhow::Result<()> {
    tracing::warn!("autostart::disable STUB — Plan 04-02 not yet implemented");
    Ok(())
}
