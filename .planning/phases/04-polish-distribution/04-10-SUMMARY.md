---
phase: 04-polish-distribution
plan: 10
subsystem: frontend-css-surface
tags:
  - gap-closure
  - css-regression
  - drag-region
  - premium-feel
  - phase4-polish
requirements:
  - DIST-04
dependency_graph:
  requires:
    - 04-09  # WebView warmup + popup_ready handshake (settings_ready invoke at line 73 stays intact)
  provides:
    - settings-window-fully-dark-surface
    - wizard-window-fully-dark-surface
    - custom-thin-scrollbar-inside-rounded-card
    - skeleton-row-dimensions-match-source-row
    - companion-header-full-drag-surface
    - settings-header-full-drag-surface
  affects:
    - companion-window-drag-behavior  # CompanionHeader.tsx attrs cascade to existing companion render path
tech_stack:
  added: []  # zero new dependencies; pure surface-layer fix
  patterns:
    - "Global html/body/#root reset (margin/padding 0, height 100%, bg #111114, overflow hidden) — canonical pattern lifted from companion.css/popup.css"
    - "height: 100% shell + flex:1 + overflow-y:auto body = bounded scroll viewport inside rounded card"
    - "::-webkit-scrollbar styling (Tauri WebView2/Chromium ships on Windows so vendor prefix is sufficient)"
    - "data-tauri-drag-region per-element attribute (Tauri 2 does NOT inherit from parent header)"
key_files:
  created: []
  modified:
    - "src/styles/settings.css"
    - "src/components/CompanionHeader.tsx"
    - "src/Settings.tsx"
decisions:
  - "Use height: 100% (not min-height: 100vh) on shells — must.haves contract requires bounded box for body's overflow-y:auto to engage"
  - "Use ::-webkit-scrollbar only (no -moz- or scrollbar-width fallback) — Tauri 2 ships Chromium-based WebView2 on Windows; no Firefox path exists in v1 (T-04G-11 accepted)"
  - "Apply position: sticky to both .settings-header and .wizard-header even though height-bounded shells make it defensive — costs nothing and matches diagnosis recommendation #6"
  - "Settings gap 32px → 24px stays on UI-SPEC 4-point scale (token: 'lg'); wizard gap left at 16px (no over-tall layout there)"
  - "Add data-tauri-drag-region to companion title + badge + settings title (3 attrs); close buttons auto-excluded by Tauri's interactive-element rule"
metrics:
  duration: "2m 14s"
  completed: "2026-05-09"
  tasks_completed: 2
  files_changed: 3
  commits: 2
---

# Phase 4 Plan 10: Settings/Wizard CSS Surface + Drag-Region Gap Closure Summary

Fixed UAT regression where Settings and First-Run Wizard windows shipped with off-white body bleed and native OS scrollbar at the window edge — caused by missing global html/body reset and `min-height: 100vh` on shells; same patch adds custom thin dark scrollbar, sticky headers, skeleton dim mirror, tightened section gap, and three missing `data-tauri-drag-region` attributes on header child elements.

## What was done

Six coordinated CSS changes to `src/styles/settings.css` (single commit) plus three drag-region attribute additions across two TSX files (single commit). No backend, no Rust compile, no new dependencies, no new design tokens.

### Task 1: src/styles/settings.css — coordinated patch (commit 9b9c89a)

| Change | What | Why |
|--------|------|-----|
| 1. Global reset | Added `html, body { margin:0; padding:0; height:100%; background:#111114; color:#F0F0F5; overflow:hidden; ... }` and `#root { width:100vw; height:100vh; }` block at top of file | Mirrors companion.css lines 2-8 + popup.css lines 3-8. The missing reset was the upstream cause of the off-white bleed (UA white body bg) AND the native OS scrollbar (unbounded body height). UI-SPEC § Inheritance contract explicitly required Phase 4 to inherit Phase 2 patterns. |
| 2. Height-bounded shells | `.settings-shell` and `.wizard-shell`: `min-height: 100vh` → `height: 100%` | Establishes a bounded box so `flex:1 + overflow-y:auto` on `.settings-body` / `.wizard-body` actually engage as scroll viewports. Without this, overflow propagated to body which spawned the OS-default scrollbar at the window edge OUTSIDE the rounded card. |
| 3. Custom scrollbar | Appended `::-webkit-scrollbar` rules (8px width, transparent track, rgba(255,255,255,0.10) thumb, hover 0.20) for both `.settings-body` and `.wizard-body` | Now that scroll lives inside the rounded card (Change 2), the WebView2 scrollbar matches the dark surface. WebKit prefix is sufficient on Tauri 2 / Windows (T-04G-11 accepted). |
| 4. Skeleton dim mirror | `.skeleton-line`: `height: 36px` → `min-height: 36px + padding: 8px + box-sizing: border-box + border-radius: 8px` | `.source-row` is `padding: 8px; min-height: 36px; border-radius: 8px` — skeleton rows must have identical rendered box so rescan resolution doesn't produce a row-height jump. |
| 5. Section gap tighten | `.settings-body { gap: 32px }` → `gap: 24px` | UI-SPEC 'lg' token (24px) on the 4-point scale. The diagnosis confirmed 3 sections + headers + skeleton state didn't fit cleanly in 580px at 32px gap; 24px restores fit without overflow. Wizard `.wizard-body` gap left at 16px (already correct). |
| 6. Sticky headers | Added `.settings-header, .wizard-header { position: sticky; top: 0; z-index: 1 }` | Defensive: with Change 2 the header is already outside the scroll viewport, but sticky guards against future content additions pushing the body past the viewport. Background is already #111114 so no transparency leak. |

Verification:
- `grep "min-height: 100vh" src/styles/settings.css` returns 0 hits.
- `grep "html, body" src/styles/settings.css` returns 1 hit.
- `grep "::-webkit-scrollbar" src/styles/settings.css` returns 8 hits (4 declaration blocks × 2 selectors each).
- `grep "position: sticky" src/styles/settings.css` returns 1 hit.
- `grep "min-height: 36px" src/styles/settings.css` returns hits (skeleton + source-row).
- `grep "gap: 24px" src/styles/settings.css` returns 1 hit (settings-body).
- `pnpm build` clean.

### Task 2: TSX drag-region attribute additions (commit aa6aa50)

| File | Change | Element |
|------|--------|---------|
| `src/components/CompanionHeader.tsx` | Added `data-tauri-drag-region` to `<div className="companion-header-title">` | line 6 |
| `src/components/CompanionHeader.tsx` | Added `data-tauri-drag-region` to `<div className="companion-header-badge">` | line 8 |
| `src/Settings.tsx` | Added `data-tauri-drag-region` to `<span className="settings-title">` | line 123 |

Tauri 2's `data-tauri-drag-region` is per-element (not inherited from parent), so the title text and badge text needed their own attributes even though the parent `<header>` and parent `<div className="settings-header">` already had it. Close buttons (`.companion-close`, `.settings-close`) are auto-excluded by Tauri's interactive-element rule and remain clickable.

Verification:
- CompanionHeader.tsx: 3 occurrences of `data-tauri-drag-region` (header + title + badge).
- Settings.tsx: 2 occurrences (header + title).
- `pnpm build` clean.

## UAT items closed

| UAT Test | Description | Status |
|----------|-------------|--------|
| Test 3 | Companion header drag — title pixels reliably grab window | Closed (drag-region added to title `<div>` + badge `<div>`) |
| Test 6 | Settings header drag — title pixels reliably grab window | Closed (drag-region added to title `<span>`) |
| Test 6 | Settings window full dark surface — no off-white bleed, no native scrollbar at window edge | Closed (CSS reset + height-bound shell + custom scrollbar) |
| Test 7 | Skeleton placeholder rows match SettingsSourceRow heights — no layout jump on rescan | Closed (skeleton dim mirror) |
| Test 14 | Wizard window dark surface (CSS portion) | Closed (same CSS file covers wizard-shell + wizard-body) |

UAT test 14's WebView warmup (settings_ready / wizard_ready handshake) remains owned by 04-09 (already landed). 04-10 closes only the CSS surface portion of test 14.

## Deviations from Plan

None — plan executed exactly as written.

The plan called out a possible Settings.tsx merge concern with 04-09, but since 04-09 had already landed (commit e405388 visible in git log) and edits a different region (line 73 useEffect, settings_ready invoke), no conflict materialized. The line 123 `<span>` edit was applied cleanly.

## Decisions Made

1. **height: 100% over 100vh** — Plan specified exactly this; chose it because `100vh` doesn't account for body margin and was part of the regression chain.
2. **Single coordinated CSS commit (Task 1) instead of 6 sub-commits** — Plan structured Task 1 as one task with 6 numbered changes; one commit captures the coordinated logical unit (the gap-closure patch). Each change has its own block comment in the CSS file noting `Phase 4 gap-closure 04-10` rationale.
3. **WebKit-only scrollbar selectors** — Plan accepted T-04G-11 (no Firefox WebView path in v1); did not add scrollbar-width / scrollbar-color Gecko fallbacks.
4. **Sticky header applied defensively** — Even though height-bound shell technically makes header sit outside scroll viewport, the defensive declaration costs nothing and matches diagnosis recommendation.

## Threat Model Compliance

All four threats from the plan's threat register are addressed or accepted as documented:

- **T-04G-09** (mitigate — future content overflow): Sticky header (Change 6) keeps chrome usable; custom scrollbar engages inside the rounded card.
- **T-04G-10** (accept — drag-region during text-select on title): Standard industry pattern (Discord/Slack/VS Code); UX gain >> minor inconvenience.
- **T-04G-11** (accept — Firefox WebView): No Firefox path in Tauri 2 on Windows; documented for future contributors.
- **T-04G-12** (accept — user theme override): No theme-loading surface in v1 (CONTEXT.md D-12).

No new threat surface introduced by this plan (pure CSS + DOM attribute additions on existing elements).

## Files Modified

- `src/styles/settings.css` — +64 lines, -4 lines (reset block + 6 logical changes)
- `src/components/CompanionHeader.tsx` — +2 modified lines (title and badge `<div>`)
- `src/Settings.tsx` — +1 modified line (settings-title `<span>`)

## Commits

| Commit | Type | Description |
|--------|------|-------------|
| `9b9c89a` | fix | settings/wizard CSS surface gaps (6 coordinated changes) |
| `aa6aa50` | fix | drag-region attrs on companion + settings header children |

## Self-Check: PASSED

- [x] `src/styles/settings.css` exists and contains html/body reset, scrollbar styling, sticky position, skeleton min-height, gap 24px, no min-height: 100vh.
- [x] `src/components/CompanionHeader.tsx` exists with 3 `data-tauri-drag-region` occurrences.
- [x] `src/Settings.tsx` exists with 2 `data-tauri-drag-region` occurrences.
- [x] Commit `9b9c89a` exists in git log.
- [x] Commit `aa6aa50` exists in git log.
- [x] `pnpm build` succeeds (verified twice — once after Task 1, once after Task 2).
