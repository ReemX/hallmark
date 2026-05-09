---
phase: 04-polish-distribution
plan: 05
subsystem: first-run-wizard
tags:
  - first-run-wizard
  - path-discovery
  - phase4-wizard
  - DIST-04

dependency_graph:
  requires:
    - "04-01a: wizard.html entry, wizard-capability JSON, first_run.rs stub, types.ts SourceStatus/DiscoveredPathsView"
    - "04-01b: wizard_dismiss command, rescan_paths command, open_wizard wired in setup()"
    - "04-04: settings.css base styles (source-row, skeleton-line, settings-pill-button), SettingsSourceRow shape"
  provides:
    - "first_run::open_wizard — WebviewWindowBuilder 480x560 borderless card, idempotent"
    - "src/FirstRunWizard.tsx — N>0 / N=0 conditional wizard page"
    - "src/components/WizardSourceRow.tsx — source row component for wizard"
    - "src/main-wizard.tsx — wizard window React entry point"
    - "src/styles/settings.css — wizard CSS block appended"
  affects:
    - "DIST-04: first-run path-discovery wizard complete"
    - "lib.rs::open_wizard no longer a stub (setup() already calls it)"

tech_stack:
  added: []
  patterns:
    - "WebviewWindowBuilder idempotent pattern (check get_webview_window first, then build)"
    - "rescan_paths on mount — React decides layout, not Rust parameter passing"
    - "wizard_dismiss as single dismiss handler for both Get started and Continue anyway — Rust gates first_run_done flag via cached_discovery"
    - "rustToView mapper in wizard mirrors Settings.tsx mapper for symmetric source-status derivation"

key_files:
  created:
    - src-tauri/src/first_run.rs
    - src/FirstRunWizard.tsx
    - src/components/WizardSourceRow.tsx
  modified:
    - src/main-wizard.tsx
    - src/styles/settings.css

decisions:
  - "_any_path_detected unused in Rust (underscore prefix silences warning) — React calls rescan_paths on mount, so React owns layout decision; this enables Rescan to flip N=0→N>0 layout without re-creating the window"
  - "Both Get started and Continue anyway invoke wizard_dismiss — Rust side (cached_discovery) determines whether first_run_done is latched; no button-identity-based branching in frontend"
  - "WizardSourceRow is a separate file from SettingsSourceRow (not a re-export) — enables future per-surface divergence (e.g., Help links in N=0 wizard rows) as a single-file change"
  - "N=0 renders Rescan + Get started + Continue anyway (3 buttons); N>0 renders Get started only — matches UI-SPEC § Interaction States"

metrics:
  duration: 2min
  completed_date: "2026-05-09"
  tasks: 2
  files: 5
---

# Phase 4 Plan 05: First-Run Wizard Summary

**One-liner:** First-run path-discovery wizard with 480x560 borderless card, N>0 (welcome + found sources) and N=0 (honest framing + rescan + continue anyway) conditional rendering, satisfying DIST-04 and D-13 through D-17.

## What Was Built

### Task 1: first_run.rs wizard window builder (3b2b68d)

`src-tauri/src/first_run.rs` replaces the Plan 04-01a stub with a full `open_wizard` implementation:

- `WebviewWindowBuilder` producing a "wizard" window at 480x560 logical px
- `decorations: false` — no OS chrome; D-13 compliance (no close button)
- `resizable: false`, `center()`, `focused: true` — wizard appears front-and-center
- Idempotent: `app.get_webview_window("wizard")` guard shows + focuses existing window instead of creating a duplicate
- `_any_path_detected` intentionally unused (underscore prefix) — React decides which layout to render after calling `rescan_paths` on mount; passing the boolean from Rust would only serve as a hint, but Rescan UX requires live re-evaluation anyway

**Wizard window dimensions + locked layout:**
- 480 × 560 logical px (UI-SPEC § Surface Specifications)
- `decorations: false` → no OS title bar or close button (D-13)
- `resizable: false` → fixed card, user cannot resize
- `always_on_top: false` → wizard does not float over other apps
- Borderless rounded card appearance via CSS `border-radius: 12px` on `.wizard-shell`

### Task 2: Wizard React page + WizardSourceRow + entry + CSS (36deb74)

**src/components/WizardSourceRow.tsx** — separate file from SettingsSourceRow for future divergence safety. Identical visual shape (same CSS classes `source-row`, `source-mark`, `source-name`) but isolated so N=0 help links or other wizard-specific row treatments can be added without touching Settings.

**src/FirstRunWizard.tsx** — full wizard page with conditional rendering:

N>0 / N=0 conditional rendering decision tree:
```
mount → invoke("rescan_paths") → rustToView() → setView()
  ↓
view?.sources.some(s => s.found) ?
  N>0: "Welcome to Hallmark" heading
       "We found these achievement sources on your system:" subheading
       Found-sources list (found=true only)
       [Get started] button (accent border)
  N=0: "No sources detected yet" heading
       "Here's what we looked for:" subheading
       All-sources list (found + not-found with detail)
       Explainer: "Hallmark watches these locations automatically..."
       [Rescan] [Get started] [Continue anyway] buttons
```

**Re-fire invariant:**
- first_run_done unset → setup() calls open_wizard → wizard opens
- first_run_done set + 0 paths detected → setup() calls open_wizard (D-14 re-fire)
- first_run_done set + ≥1 path detected → no wizard

**Continue-anyway escape-hatch UX:**
Both `Get started` and `Continue anyway` call `invoke("wizard_dismiss")`. The Rust-side `wizard_dismiss` command checks `cached_discovery` — if any paths are present, it writes `first_run_done=1`; if no paths, it closes the window without latching the flag. This means "Continue anyway" on a zero-path machine preserves the re-fire behavior (D-14) without requiring two separate Tauri commands. The distinction is purely informational UX — "Get started" signals readiness, "Continue anyway" signals awareness — but mechanically they're identical invocations.

**src/main-wizard.tsx** — replaces the Plan 04-01a stub (which rendered `<div />`) with `FirstRunWizardRoot` + `settings.css` import. 10 lines.

**src/styles/settings.css** — wizard CSS appended (existing update modal rules untouched):
- `.wizard-shell` — full-height dark card, font stack, border-radius 12px
- `.wizard-header` — 48px drag region, no close button slot
- `.wizard-title` — 16px/600 heading
- `.wizard-body` — flex column, 32/24px padding, 16px gap
- `.wizard-subheading` — 14px/400, rgba(240,240,245,0.85)
- `.wizard-explainer` — 14px/400, rgba(240,240,245,0.55), N=0 only
- `.wizard-buttons` — column flex, 8px gap, stretch alignment
- `.wizard-cta-primary` — `border: 1px solid rgba(120,220,255,0.40)` — only wizard button with accent border (UI-SPEC)
- `.wizard-cta-secondary` — no border/background, text-secondary color, 14px/400

## Verification Results

- `cargo build --lib`: PASS
- `cargo build --bin hallmark`: PASS
- `pnpm build`: PASS — 4 entry bundles (wizard.html 3.39KB, popup.html, settings.html, index.html)
- All acceptance criteria checks: PASS

## Deviations from Plan

None — plan executed exactly as written.

Both tasks implemented as specified in the plan action blocks. The `_any_path_detected` underscore annotation is documented in the code and matches the plan's rationale.

## Known Stubs

None. All stubs from this plan's scope have been replaced with functional implementations:
- `src-tauri/src/first_run.rs::open_wizard` — fully implemented (this plan)
- `src/main-wizard.tsx` — replaced WizardStub with FirstRunWizardRoot (this plan)

## Threat Surface Scan

No new threat surface beyond the plan's `<threat_model>`:
- T-04-22: wizard shows source NAMES only, not paths — satisfied (WizardSourceRow renders only `source.name` and `source.detail` strings)
- T-04-23: frontend bypass is accepted per design (UX feature, not a security boundary)
- T-04-24: no close button is intentional; OS Alt+F4 + tray Quit remain as escape valves

No new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check: PASSED

Files created/exist:
- src-tauri/src/first_run.rs: FOUND (36 lines, WebviewWindowBuilder, 480x560, no STUB)
- src/FirstRunWizard.tsx: FOUND (160 lines, N>0/N=0 rendering, invoke wizard_dismiss/rescan_paths)
- src/components/WizardSourceRow.tsx: FOUND (17 lines, source-row/source-mark/source-name)
- src/main-wizard.tsx: FOUND (6 lines, FirstRunWizardRoot entry)
- src/styles/settings.css: FOUND (wizard-shell, wizard-cta-primary with accent border)

Commits exist:
- 3b2b68d: feat(04-05): implement first_run::open_wizard window builder
- 36deb74: feat(04-05): wizard React page, WizardSourceRow, entry point, CSS
