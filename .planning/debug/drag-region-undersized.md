---
status: investigating
trigger: "drag-region-undersized — companion + settings header strips have unreliable drag regions; title text not draggable"
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: H1 (CONFIRMED) — `data-tauri-drag-region` is on the parent header, but Tauri's drag-region detection only triggers on direct hits to elements that **carry the attribute themselves**. Children (the title `<div>` / `<span>`, the badge `<div>`) do NOT inherit the attribute and therefore are not part of the drag surface. Title pixels = title element = no drag.
test: Read both header components and CSS; cross-reference Tauri 2 documentation for `data-tauri-drag-region` semantics
expecting: confirm that child elements without the attribute are excluded from drag detection
next_action: report findings to caller (goal: find_root_cause_only)

## Symptoms

expected: The full 48 px header strip — including the title text pixels and any non-button gaps — should grab the window for drag.
actual: Both windows have header strips where some pixels grab and drag while others (notably the title text itself) do nothing. Companion: "kinda flaky". Settings: title text "Settings" is not draggable but chrome around it is.
errors: None.
reproduction: cargo tauri dev → click tray icon (companion shows) → try to drag by clicking and holding on "Hallmark" title pixels. Open settings via tray → try to drag by clicking the "Settings" title pixels.
started: Discovered during Phase 4 UAT tests 3 + 6 on 2026-05-09.

## Eliminated

(none yet — H1 confirmed first)

## Evidence

- timestamp: 2026-05-09T00:00:00Z
  checked: src/components/CompanionHeader.tsx (lines 1-20)
  found: |
    The companion `<header>` has `data-tauri-drag-region` on the parent only.
    Children:
      - `<div className="companion-header-title">{gameName}</div>`  ← NO drag attribute
      - `<div className="companion-header-badge">…</div>`            ← NO drag attribute
      - `<button className="companion-close">×</button>`             ← correctly excluded (button)
  implication: Title and badge `<div>`s sit on top of the header but lack the drag attribute. They block the parent's drag detection on those pixel rows.

- timestamp: 2026-05-09T00:00:00Z
  checked: src/Settings.tsx (lines 117-128)
  found: |
    Settings header `<div className="settings-header" data-tauri-drag-region>` has children:
      - `<span className="settings-title">Settings</span>`  ← NO drag attribute
      - `<button className="settings-close">×</button>`     ← correctly excluded (button)
    Layout uses `justify-content: space-between` so the title sits on the left edge and the close button on the right; the gap between them is the only direct hit on the header `<div>` itself.
  implication: User reports "title text 'Settings' is not draggable but chrome around it is" — exactly matches: clicks on the `<span>` hit the span, not the parent. Only the empty flex gap pixels register as drag.

- timestamp: 2026-05-09T00:00:00Z
  checked: src/styles/companion.css (lines 11-30) and src/styles/settings.css (lines 15-33)
  found: |
    No `pointer-events` overrides on either header or its children.
    No `-webkit-app-region` rules (Tauri 2 uses the `data-tauri-drag-region` attribute, not the legacy CSS property — confirming H4 is N/A here).
    Both headers use `display: flex` with the title as the first child. In companion the title has `flex: 1` so it spans most of the header width — explaining "kinda flaky": only the small pixel area covered by the badge OR the gaps before/after the title text register drag, while the rest of the title element does not.
  implication: Pure markup issue — the attribute simply is not propagated to non-button descendants. No CSS layer is interfering.

- timestamp: 2026-05-09T00:00:00Z
  checked: Tauri 2 official docs behavior — `data-tauri-drag-region` is a marker attribute that Tauri's drag handler reads from the DOM element under the cursor. Children do NOT inherit; each element that should be draggable must carry the attribute itself (interactive controls like `<button>`, `<input>`, `<a>` automatically opt out).
  found: This is documented Tauri 2 behavior. Recommended pattern: place the attribute on every non-interactive descendant of the header, OR (simpler) add `data-tauri-drag-region` to the title element directly so its pixels are part of the drag surface.
  implication: The fix is purely additive — sprinkle the attribute on the title (and badge, for companion).

## Resolution

root_cause: |
  In both CompanionHeader.tsx and Settings.tsx, `data-tauri-drag-region` is applied ONLY to the outermost header element. Tauri 2 does NOT propagate the attribute to descendants — it is checked on the exact DOM element under the cursor. Since the title text sits inside a child `<div>` (companion) or `<span>` (settings), pixels covering the title do not match `data-tauri-drag-region` and are therefore not part of the drag surface. The badge in the companion header has the same issue. Only the small gap pixels between flex children fall directly on the parent header element and are draggable — explaining "flaky" + "title text won't drag" exactly.

fix: |
  Add `data-tauri-drag-region` to non-interactive descendants of each header.
  Companion (src/components/CompanionHeader.tsx):
    - Line 6: `<div className="companion-header-title" data-tauri-drag-region>{gameName}</div>`
    - Line 8: `<div className="companion-header-badge" data-tauri-drag-region>{sessionEarned} earned this session</div>`
  Settings (src/Settings.tsx):
    - Line 120: `<span className="settings-title" data-tauri-drag-region>Settings</span>`
  No CSS changes required. The close `<button>` elements are auto-excluded by Tauri's interactive-element rule and stay clickable.

verification: (deferred — goal is find_root_cause_only)
files_changed: []
