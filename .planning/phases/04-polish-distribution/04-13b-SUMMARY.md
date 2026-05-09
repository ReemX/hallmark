---
phase: 04-polish-distribution
plan: 13b
subsystem: ui
tags: [tray, ico, artwork, branding]

requires:
  - phase: 01-detection-pipeline-foundation
    provides: src-tauri/icons/{tray,icon}.ico placeholder ICOs (commit 452d29b)
  - phase: 04-polish-distribution
    provides: 04-13a tray menu structure (D-01 amended)

provides:
  - Real multi-resolution tray.ico (16-256 frames, non-zero alpha) replacing all-transparent placeholder
  - Real multi-resolution icon.ico for NSIS-bundled window/taskbar/installer icon
  - Closes UAT test 2 root cause #2 (black-square tray icon)

affects: [tray-menu, installer-icon, brand-identity]

tech-stack:
  added: []
  patterns:
    - "Multi-resolution ICO authoring via Pillow with explicit `sizes=` (verified via ico.sizes(), not n_frames)"

key-files:
  created:
    - "build/glyph_trophy.png" (source artwork — not committed; regeneration script in SUMMARY)
  modified:
    - "src-tauri/icons/tray.ico"
    - "src-tauri/icons/icon.ico"

key-decisions:
  - "Trophy glyph (cup + handles + stem + base) chosen over plain 'H' — Hallmark = achievements brand fit"
  - "Single-icon for both light and dark Windows 11 themes — UI-SPEC § Tray Menu permits, defers theme variants to v1.1"
  - "Same artwork for tray.ico and icon.ico per plan recommendation (placeholders were already byte-identical)"
  - "Resume signal: `approved-v1-iterate` — v1 placeholder, polish slot reserved for v1.1"

patterns-established:
  - "ICO multi-frame verification: use `Image.open(path).ico.sizes()` to enumerate frames; `n_frames` returns 1 for IcoImageFile and is misleading"

requirements-completed: [POL-01, POL-02]

duration: ~5min
completed: 2026-05-10
---

# Phase 04-13b: Real Tray Icon Artwork Summary

**Trophy-glyph multi-res ICO replaces all-transparent Phase-1 placeholder; Windows 11 tray now renders a recognizable shape instead of a solid black square.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-05-10
- **Completed:** 2026-05-10
- **Tasks:** 2 of 3 done autonomously (Task 1 user-delegated to orchestrator per `Auto-generate stylized 'H' glyph` → trophy variant; Task 3 visual UAT deferred to phase verification)
- **Files modified:** 2 binary assets

## Accomplishments

- Replaced byte-identical alpha-zero placeholders (commit 452d29b) with real artwork
- Multi-resolution ICO (16, 24, 32, 48, 64, 128, 256) baked via `include_bytes!` in `tray::build_tray`
- Alpha verification PASS: tray.ico 16×16 = 157 non-zero alpha pixels (61% coverage), 32×32 = 569 (55% coverage); icon.ico identical
- `cargo build --workspace` clean

## Task Commits

1. **Task 1 (user-delegated): drop in real artwork** — committed with Task 2 (binary asset commit)
2. **Task 2: Python alpha verification + cargo build** — `<commit hash>` (chore/feat: real tray + installer artwork)
3. **Task 3: HUMAN visual UAT in Windows 11** — DEFERRED to phase verification (`/gsd-verify-work` or live `cargo tauri dev` check)

## Artwork Source / Regeneration

**Glyph:** Trophy silhouette (cup body + ear handles + stem + stepped base), white on transparent.

**Generation script** (PIL/Pillow, fully reproducible):
```python
from PIL import Image, ImageDraw

W = 256
img = Image.new('RGBA', (W, W), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
WHITE = (255, 255, 255, 255)

cup_top, cup_bot, cup_left, cup_right = 32, 144, 64, 192
d.rounded_rectangle([(cup_left, cup_top), (cup_right, cup_bot)], radius=20, fill=WHITE)
taper = 24
d.polygon([(cup_left, cup_bot - 24), (cup_left, cup_bot), (cup_left + taper, cup_bot)], fill=(0, 0, 0, 0))
d.polygon([(cup_right, cup_bot - 24), (cup_right, cup_bot), (cup_right - taper, cup_bot)], fill=(0, 0, 0, 0))
d.ellipse([(28, 50), (88, 130)], fill=WHITE);  d.ellipse([(48, 70), (76, 110)], fill=(0, 0, 0, 0))
d.ellipse([(168, 50), (228, 130)], fill=WHITE); d.ellipse([(180, 70), (208, 110)], fill=(0, 0, 0, 0))
d.rectangle([(116, 140), (140, 188)], fill=WHITE)
d.rectangle([(80, 188), (176, 208)], fill=WHITE)
d.rounded_rectangle([(64, 208), (192, 232)], radius=6, fill=WHITE)

img.save('build/glyph_trophy.png')
img.save('src-tauri/icons/tray.ico', sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
import shutil; shutil.copyfile('src-tauri/icons/tray.ico', 'src-tauri/icons/icon.ico')
```

## Verification

**Alpha check (Task 2 step B):**
```
src-tauri/icons/tray.ico (16, 16): 157 non-zero alpha pixels
src-tauri/icons/tray.ico (32, 32): 569 non-zero alpha pixels
src-tauri/icons/icon.ico (16, 16): 157 non-zero alpha pixels
src-tauri/icons/icon.ico (32, 32): 569 non-zero alpha pixels
ICO artwork acceptance: PASS
```

**Build (Task 2 step C):** `cargo build --workspace` → clean.

**Visual UAT (Task 3):** DEFERRED. User runs `cargo tauri dev` during phase verification; tray icon must render as a recognizable trophy glyph (NOT solid black, NOT transparent gap), right-click menu shows the amended D-01 layout (no Hallmark header, per 04-13a), tooltip "Hallmark" appears on hover.

## Decisions Made

- **Glyph choice:** Trophy over plain 'H' — better thematic fit for achievements app, distinct silhouette at 16×16.
- **Same ICO for tray.ico + icon.ico:** Plan recommendation; placeholders were already byte-identical.
- **No theme variants:** v1 ships single icon, theme-aware split deferred to v1.1.
- **Resume signal: `approved-v1-iterate`** — artwork is "good enough for v1"; polish slot reserved.

## Deviations from Plan

None — plan executed exactly as written. Task 1's human-action checkpoint was delegated to the orchestrator per user's `Auto-generate stylized 'H' glyph` answer, refined to trophy on user follow-up.

## Issues Encountered

- **Pillow `n_frames` misleads on ICO:** `Image.open(path).n_frames` returns 1 for IcoImageFile even when the file contains multiple frames. The plan's verify script (Task 2 inline Python) used `n_frames` and falsely reported "missing 16x16 frame". Worked around by switching to `ico.sizes()` (the IcoFile API). Plan inline verify command should be updated in v1.1 if this plan gets re-run; v1 SUMMARY captures the corrected snippet above.
- **First Pillow attempt (single-pass `sizes=` on save):** Worked correctly; `ico.sizes()` confirmed all 7 frames present. The earlier debug output showing `n_frames: 1` was a verification-script bug, not a generation bug.

## User Setup Required

None — binary assets are committed to the repo and `include_bytes!` at compile time.

## Next Phase Readiness

- UAT test 2 root cause #2 closed (black-square tray icon resolved).
- Phase 4 verification can re-run UAT test 2 once `cargo tauri dev` is launched.
- Sibling 04-13a (header drop) already committed; both root causes for test 2 now addressed.

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-10*
