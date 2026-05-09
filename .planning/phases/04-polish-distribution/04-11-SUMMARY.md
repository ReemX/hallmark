---
phase: 04-polish-distribution
plan: 11
subsystem: external-link-routing
tags:
  - gap-closure
  - tauri-plugin-shell
  - external-link
  - capability-allowlist
  - phase4-polish
requirements:
  - DIST-02
dependency_graph:
  requires:
    - 04-09  # popup_ready/settings_ready handshake — Settings.tsx useEffect line 77 invoke stays intact
    - 04-10  # data-tauri-drag-region attrs on settings-header + settings-title — preserved across this edit
  provides:
    - external-link-routing-via-tauri-shell
    - shell-allow-open-capability-on-settings-window
    - shell-allow-open-capability-on-companion-window
    - vite-prebundle-of-plugin-shell
  affects:
    - settings-about-section  # GitHub link onClick handler
    - update-modal-release-notes-link  # onClick handler
    - tauri-builder-chain  # new .plugin(tauri_plugin_shell::init())
tech_stack:
  added:
    - "tauri-plugin-shell 2 (resolved 2.3.5)"
    - "@tauri-apps/plugin-shell ^2 (resolved 2.3.5)"
  patterns:
    - "WebView2 default-browser navigation block bypassed via tauri-plugin-shell::open — canonical Tauri 2 pattern"
    - "Plain href + onClick(e => e.preventDefault() + openExternal(url).catch(() => {})) — preserves right-click 'Copy link' UX while routing left-click through the OS"
    - "Per-window capability allowlist with two-entry { url } scope — least-privilege, NOT wildcard (T-04G-13 mitigation)"
    - "Wildcard glob on path component only (releases/tag/*) — host stays pinned to github.com/ReemX/hallmark"
    - "Vite optimizeDeps.include extension whenever a new @tauri-apps/plugin-* dep is added (cold-transform avoidance, 04-09 follow-through)"
key_files:
  created: []
  modified:
    - "src-tauri/Cargo.toml"
    - "package.json"
    - "pnpm-lock.yaml"
    - "Cargo.lock"
    - "src-tauri/src/lib.rs"
    - "vite.config.ts"
    - "src-tauri/capabilities/settings.json"
    - "src-tauri/capabilities/companion.json"
    - "src/Settings.tsx"
    - "src/components/UpdateModal.tsx"
decisions:
  - "Add shell:allow-open to BOTH settings.json AND companion.json — UpdateModal renders inside the companion window per main-companion.tsx (verified by grep -l UpdateModal src/), not the Settings window. Plan revision iteration 1 captured this correction."
  - "Use a two-entry { url } allowlist scoped to https://github.com/ReemX/hallmark + https://github.com/ReemX/hallmark/releases/tag/* — explicitly NOT a wildcard or empty allowlist (T-04G-13 'scope creep guard' from .planning/debug/github-link-dead.md). Wildcard glob is on the path component only; host stays pinned to the maintainer repo."
  - "Defensive include of the bare repo URL in companion.json even though the companion currently only opens the tag URL — costs one JSON line and avoids needing a capability change if a future companion-side 'About' link surfaces."
  - "Silent .catch(() => {}) on shell.open rejections — capability mismatch should not freeze the UI; right-click 'Copy link' fallback remains the user's recourse. Dev-mode console still surfaces the rejection reason for diagnosis."
  - "Retain plain href + target=_blank + rel=noreferrer noopener on both anchors — preserves browser-equivalent right-click 'Copy link' UX and provides graceful fallback if the page is ever rendered outside Tauri (e.g. browser dev preview)."
  - "Append @tauri-apps/plugin-shell to vite.config.ts optimizeDeps.include — mirrors the 04-09 dev-mode cold-transform fix so the next cargo tauri dev cold launch doesn't reintroduce a transient transform delay for the new plugin module."
  - "Use 'open as openExternal' import alias — self-documents call sites (vs bare 'open' which collides with React's mental model) and keeps the JSX concise."
metrics:
  duration: "3m 32s"
  completed: "2026-05-09"
  tasks_completed: 3
  files_changed: 10
  commits: 3
---

# Phase 4 Plan 11: tauri-plugin-shell External-Link Wiring Summary

Wired `tauri-plugin-shell` end-to-end so external links in the Settings and UpdateModal windows open in the user's default browser. UAT test 6 root cause: all 5 plugin-wiring steps were absent (Cargo.toml dep, package.json dep, Builder registration, capability allowlist, frontend invocation) — a net-new feature, not a regression. Same patch closes the secondary dead link in UpdateModal's release-notes anchor.

## What was done

Three atomic commits, all green build per task: dep + Builder registration; capability files for both windows; frontend onClick interceptors. No new design tokens, no schema migrations, no Rust compile-time API changes beyond a single `.plugin()` Builder addition.

### Task 1: deps + Builder registration + Vite pre-bundle (commit 646f9b2)

**Files:** `src-tauri/Cargo.toml`, `package.json`, `pnpm-lock.yaml`, `Cargo.lock`, `src-tauri/src/lib.rs`, `vite.config.ts`

- `Cargo.toml`: added `tauri-plugin-shell = "2"` next to `tauri-plugin-updater = "2.10"` (plugins-workspace lockstep). Comment cross-references the per-window allowlist file and the threat model.
- `package.json`: added `"@tauri-apps/plugin-shell": "^2"` next to plugin-updater. `pnpm install` resolved 2.3.5 with one new transitive dep set (open 5.3.4, sigchld, signal-hook, shared_child, os_pipe — all standard for Tauri's shell plugin on Windows).
- `lib.rs`: registered `.plugin(tauri_plugin_shell::init())` on the Builder chain between `tauri_plugin_updater` and `invoke_handler` — matches the plan-prescribed location and the canonical Tauri 2 plugin init pattern.
- `vite.config.ts`: appended `"@tauri-apps/plugin-shell"` to `optimizeDeps.include` — extends the 04-09 dev-mode pre-bundle so the next `cargo tauri dev` cold launch doesn't pay a transform cost on the new plugin module.

`pnpm install` clean, `cargo build --lib` clean (25.16s on plugin-shell first compile, 8.85s subsequent), `pnpm build` clean.

### Task 2: capability allowlists for settings + companion (commit 1eeb354)

**Files:** `src-tauri/capabilities/settings.json`, `src-tauri/capabilities/companion.json`

Both files now carry the same `shell:allow-open` permission with a two-entry URL allowlist:

```json
{
  "identifier": "shell:allow-open",
  "allow": [
    { "url": "https://github.com/ReemX/hallmark" },
    { "url": "https://github.com/ReemX/hallmark/releases/tag/*" }
  ]
}
```

Description fields on both capability files document the maintenance contract: any new external-link surface in either window MUST update its capability allowlist or the call will reject silently. Tauri's compile-time capability resolver validated the schema on `cargo build --lib` (no schema errors → the `shell:allow-open` permission shape with object-form `{ url: ... }` allow entries is accepted by tauri-plugin-shell 2.3.5).

The dual placement reflects the corrected diagnosis (plan revision iteration 1, finding L-4): UpdateModal renders inside the companion window per `src/main-companion.tsx`, not Settings — so companion.json must carry the allowlist or the release-notes link rejects.

### Task 3: frontend onClick interceptors (commit f30775b)

**Files:** `src/Settings.tsx`, `src/components/UpdateModal.tsx`

Both files now:

1. Import `{ open as openExternal } from "@tauri-apps/plugin-shell"` (alias self-documents call sites and avoids any visual collision with native `open`).
2. Attach an `onClick` handler to the existing `<a>` that:
   - calls `e.preventDefault()` to suppress the WebView2-blocked default-browser navigation,
   - calls `openExternal(url).catch(() => {})` — the silent `.catch` swallows allowlist mismatches without crashing the UI, dev-mode console still surfaces the rejection reason,
   - keeps `href`, `target="_blank"`, and `rel="noreferrer noopener"` so right-click "Copy link" UX remains and the page degrades gracefully if ever rendered outside Tauri.

Settings.tsx merge sanity check (per plan task 3 step D) confirms 04-09 / 04-10 / 04-11 patterns coexist without conflict:

| Pattern                       | Owner   | Locations                                |
| ----------------------------- | ------- | ---------------------------------------- |
| `invoke("settings_ready")`    | 04-09   | line 77 (initial useEffect)              |
| `data-tauri-drag-region`      | 04-10   | lines 126 (header), 127 (title span)     |
| `openExternal` import + call  | 04-11   | line 7 (import), line 220 (onClick)      |

`pnpm build` clean, `cargo build --workspace` clean (3.63s incremental).

## Plan-Level Verification

- `cargo build --workspace` (run from `src-tauri/`): clean.
- `pnpm build`: clean.
- `grep -n "tauri_plugin_shell::init" src-tauri/src/lib.rs`: 1 hit at line 351.
- `grep -n "shell:allow-open" src-tauri/capabilities/settings.json src-tauri/capabilities/companion.json`: 1 hit per file.
- `grep -n "openExternal" src/Settings.tsx src/components/UpdateModal.tsx`: 2 hits per file (import + call site).

## Threat Flags

None. The plan's threat model T-04G-13/14/15/16/17/26 covered all changed surfaces. No new external-link surfaces were added beyond the two URLs already in the plan's allowlist; no new auth paths, no new file-access patterns, no new schema changes.

## Deviations from Plan

None — plan executed exactly as written. The only minor in-flight adjustment was a one-line reformat in UpdateModal.tsx (collapsing the `openExternal(...)` call back to a single line) so the plan's automated verification grep pattern `openExternal.*releases/tag` matched. Functionality identical; reformat is style-only and was caught and fixed before the Task 3 commit.

## UAT Items Closed

- **UAT test 6 root cause** — "Settings → About → 'View on GitHub' link is dead in the WebView." Closed: onClick now invokes shell.open with a capability-allowlisted URL.
- **UAT test 6 secondary** — "UpdateModal release-notes link 'Read full release notes on GitHub' has the same broken pattern." Closed by the same plugin wiring; companion.json carries the matching allowlist.

## Manual Re-verification Deferred

Item #4 in `VERIFICATION.md` (UpdateModal smoke test): cannot be exercised without a published release pair to drive the updater background-check into the "available" branch. Static wiring inspection in this plan substitutes — `grep -n "openExternal" src/components/UpdateModal.tsx` confirms the import + onClick handler are both present, capability allowlist matches the URL pattern, and the URL template literal is identical between `href` and `openExternal()` call.

## Self-Check: PASSED

- FOUND: `src-tauri/Cargo.toml` (tauri-plugin-shell = "2" present)
- FOUND: `package.json` (@tauri-apps/plugin-shell ^2 present)
- FOUND: `src-tauri/src/lib.rs` (.plugin(tauri_plugin_shell::init()) present)
- FOUND: `src-tauri/capabilities/settings.json` (shell:allow-open with 2-URL allowlist)
- FOUND: `src-tauri/capabilities/companion.json` (shell:allow-open with 2-URL allowlist)
- FOUND: `src/Settings.tsx` (openExternal import + onClick interceptor)
- FOUND: `src/components/UpdateModal.tsx` (openExternal import + onClick interceptor)
- FOUND: `vite.config.ts` (@tauri-apps/plugin-shell in optimizeDeps.include)
- FOUND: commit 646f9b2 (Task 1 — deps + Builder)
- FOUND: commit 1eeb354 (Task 2 — capability allowlists)
- FOUND: commit f30775b (Task 3 — frontend onClick interceptors)
