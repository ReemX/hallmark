---
status: diagnosed
trigger: "github-link-dead — Settings About 'View on GitHub' click does nothing"
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: H1 (CONFIRMED) — tauri-plugin-shell is not wired up at all. ALL 5 wiring steps absent.
test: Inspected the 5 wiring locations
expecting: Find missing pieces — found ALL pieces missing
next_action: Return ROOT CAUSE FOUND with minimal recommendation (least-privilege capability)

## Symptoms

expected: Clicking "View on GitHub" in Settings.About opens https://github.com/ReemX/hallmark in the user's default browser.
actual: Click does nothing — no browser opens, no console error visible to the user.
errors: None visible to user (browser console errors may exist; check WebView devtools).
reproduction: cargo tauri dev → tray → Settings → scroll to About panel → click "View on GitHub".
started: 2026-05-09 (Phase 4 UAT test 6)

## Eliminated

(none — H1 confirmed on first pass)

## Evidence

- timestamp: 2026-05-09
  checked: src/Settings.tsx (About section, lines 200–215)
  found: Plain `<a href="https://github.com/ReemX/hallmark" target="_blank" rel="noreferrer noopener">View on GitHub</a>` — no onClick, no shell.open invocation
  implication: WebView2 silently blocks default browser navigation for top-level navigations + new-window requests; no listener intercepts the click

- timestamp: 2026-05-09
  checked: package.json (frontend deps)
  found: Only `@tauri-apps/api`, `@tauri-apps/plugin-updater`, `framer-motion`, `react`, `react-dom` listed. NO `@tauri-apps/plugin-shell`.
  implication: Frontend cannot import { open } even if it tried — module not installed

- timestamp: 2026-05-09
  checked: src-tauri/Cargo.toml (Rust deps)
  found: tauri 2.11, tauri-plugin-updater 2.10 present. NO `tauri-plugin-shell`.
  implication: Backend has no shell plugin to register

- timestamp: 2026-05-09
  checked: src-tauri/src/lib.rs Builder chain (line 261-273)
  found: Only `.plugin(tauri_plugin_updater::Builder::new().build())` registered. No shell plugin init.
  implication: Even if Cargo dep existed, the plugin would not be loaded into the Tauri runtime

- timestamp: 2026-05-09
  checked: src-tauri/capabilities/settings.json
  found: Permissions limited to core:default, core:event:*, core:window:* (show/hide/close/start-dragging), updater:default. NO shell:allow-open.
  implication: Settings webview has no permission to invoke any shell command — invoke('plugin:shell|open',...) would be denied even if everything else were wired

- timestamp: 2026-05-09
  checked: Codebase-wide ripgrep for `tauri-plugin-shell|tauri_plugin_shell|plugin-shell|shell:allow-open|shell.open`
  found: Zero hits in code (only mentions are in this debug file and the UAT report describing the gap)
  implication: Plugin has never been wired — this is a net-new addition, not a regression

- timestamp: 2026-05-09
  checked: src/components/UpdateModal.tsx (line 65–72)
  found: SECOND dead external link — `<a href="https://github.com/ReemX/hallmark/releases/tag/v{version}" target="_blank">Read full release notes on GitHub</a>`. Same `<a target=_blank>` pattern, same WebView block.
  implication: Fix should also cover this link (one capability allowlist, one openExternal helper, both call sites updated)

## Resolution

root_cause: The "View on GitHub" link is a plain `<a target="_blank">`. Tauri 2 WebViews block default-browser navigation: opening external URLs requires `tauri-plugin-shell`'s `open` command, which depends on Cargo dep + npm dep + Builder registration + capability permission + frontend invocation. ALL FIVE are missing — the wiring was never added in Phase 4. UpdateModal.tsx contains a second dead link with the same pattern.

fix: (find_root_cause_only mode — not applied)

verification: (find_root_cause_only mode — not applied)

files_changed: []

## Recommended Fix Direction (for plan-phase --gaps)

Wire all 5 steps with a least-privilege allowlist (only the two GitHub URLs the app actually opens):

1. **Cargo.toml** (src-tauri/Cargo.toml): add `tauri-plugin-shell = "2"`

2. **package.json** (frontend): add `"@tauri-apps/plugin-shell": "^2"`

3. **lib.rs** (src-tauri/src/lib.rs): inside `tauri::Builder::default()` chain (alongside the existing `.plugin(tauri_plugin_updater::Builder::new().build())`), add `.plugin(tauri_plugin_shell::init())`

4. **settings.json capability** (src-tauri/capabilities/settings.json): add a least-privilege `shell:allow-open` permission scoped to the two GitHub URLs:
   ```json
   {
     "identifier": "shell:allow-open",
     "allow": [
       { "url": "https://github.com/ReemX/hallmark" },
       { "url": "https://github.com/ReemX/hallmark/releases/tag/*" }
     ]
   }
   ```
   Add the same permission entry to `companion.json` and any other capability file that also needs to open external URLs (currently only settings + the update modal are affected; UpdateModal renders inside the Settings window, so settings.json alone covers both call sites).

5. **Frontend** (src/Settings.tsx + src/components/UpdateModal.tsx): replace `<a href=...>` with a button (or `<a>` + onClick + preventDefault) that calls `await open(url)` from `@tauri-apps/plugin-shell`. Keep visible styling identical so the UI doesn't shift.

   Example (Settings.tsx):
   ```tsx
   import { open } from "@tauri-apps/plugin-shell";
   ...
   <a
     className="settings-link"
     href="https://github.com/ReemX/hallmark"
     onClick={(e) => {
       e.preventDefault();
       open("https://github.com/ReemX/hallmark").catch(() => {});
     }}
   >
     View on GitHub
   </a>
   ```
   The plain href is retained as a non-Tauri fallback (right-click → "Copy link" still works) but the click is intercepted.

**Scope creep guard:** Do NOT register `shell:allow-open` with an empty/wildcard allow list. The Tauri shell.open scope is a known footgun — an unscoped allow list lets any compromised page in the WebView execute arbitrary OS commands via the URL parameter on Windows. The two-entry allowlist above is sufficient for current call sites.
