---
status: diagnosed
trigger: "tray-menu-extra-header-and-black-icon"
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: Both root causes confirmed.
test: complete
expecting: complete
next_action: Return diagnosis to caller (find_root_cause_only mode).

## Symptoms

expected:
  (1) Tray menu shows: Show companion / Fire test popup / Settings / Start with Windows / Quit (no header).
  (2) Tray icon shows recognizable Hallmark glyph in Windows 11 notification area.
actual:
  (1) Menu shows extra disabled "Hallmark" header item at top.
  (2) Tray icon renders as solid black box.
errors: None.
reproduction: cargo tauri dev, observe tray icon (black box) and right-click menu (extra header).
started: Discovered Phase 4 UAT test 2 on 2026-05-09.

## Eliminated

- hypothesis: Tauri 2's TrayIconBuilder auto-injects a title/header item from `tooltip()`.
  evidence: tray.rs line 62 explicitly creates `MenuItem::with_id(app, "header", "Hallmark", false, ...)` and line 77 places it as the first item in the MenuBuilder. Also tray.rs lines 3-14 doc comment treats the "Hallmark" header as part of D-01. The header is hand-coded, not auto-injected. The tooltip() call is a separate hover-text, not a menu item.
  timestamp: 2026-05-09

- hypothesis: tray.ico lacks a 16x16 frame, forcing Windows to downsample 256x256 with bad alpha.
  evidence: ICO header parse shows 6 frames at 16/24/32/48/64/256, all 32bpp. 16x16 frame is present (offset=4366 size=1128B).
  timestamp: 2026-05-09

- hypothesis: icon.ico looks fine while tray.ico is broken — they differ.
  evidence: `ls -la` shows both files identical size (304886 B). 04-02 SUMMARY confirms tray.ico is a copy. They are byte-identical.
  timestamp: 2026-05-09

## Evidence

- timestamp: 2026-05-09
  checked: src-tauri/src/tray.rs build_menu()
  found: |
    Lines 62 + 77 explicitly add a disabled MenuItem with id "header" and text "Hallmark" as the first menu item, then a separator. Lines 3-14 doc comment claims this is the D-01 spec. Verified with grep — no other source of the menu item.
  implication: |
    Defect 1 root cause: hand-coded header item. Removing lines 62, 77, and the orphan separator is the minimal fix. (Defect 1 also has a SPEC contradiction — see next evidence entry.)

- timestamp: 2026-05-09
  checked: .planning/phases/04-polish-distribution/04-CONTEXT.md (D-01 source) vs 04-UAT.md (test 2 expected)
  found: |
    04-CONTEXT.md lines 28-38 LOCKS D-01 as: "Hallmark / sep / Show companion / Fire test popup / sep / Settings… / ☑ Start with Windows / sep / Quit" (i.e. WITH a Hallmark header item at top).
    04-UAT.md test 2 expected line 21 says: "Show companion, Fire test popup, Settings, Start with Windows, Quit. No other items." (i.e. WITHOUT a Hallmark header).
  implication: |
    Spec contradiction. tray.rs faithfully implements 04-CONTEXT D-01 (the locked spec). The UAT test 2 was authored against a different, simpler reading. The user's reported defect ("extra Hallmark item not in D-01") aligns with the UAT reading. Resolution requires the user/spec owner to decide:
    A) Keep the header (matches 04-CONTEXT D-01 lock; UAT test 2 must be updated). Code is correct.
    B) Remove the header (matches UAT and user expectation; 04-CONTEXT D-01 must be updated). Code change is small.
    Recommendation: Option B. The header is functionally inert (disabled), adds visual clutter, and is uncommon in modern tray menus (Discord/Slack/Steam don't have it). The "Hallmark" identification is already provided by the tooltip on the icon and the EXE name in process lists.

- timestamp: 2026-05-09
  checked: src-tauri/icons/tray.ico binary structure (Python ICO parser + PIL)
  found: |
    ICO has 6 frames at 16x16, 24x24, 32x32, 48x48, 64x64, 256x256 — all 32bpp BGRA + 1-bit AND mask.
    Every single frame contains 100% transparent pixels: alpha=0 for ALL pixels at ALL resolutions.
    16x16 frame raw bytes: XOR mask is 256x BGRA(0,0,0,0); AND mask is all zeros (0 = "use the XOR pixel").
    Frame[0] (256x256) alpha>0 = 0 / 65536
    Frame[5] (16x16)   alpha>0 = 0 / 256
  implication: |
    Defect 2 root cause: tray.ico is a fully-transparent placeholder ICO — it contains zero glyph data. There is no Hallmark artwork in any of the 6 frames.

    Why this renders as solid black in the Windows 11 tray:
    - Modern tray rendering would show the icon area as fully transparent (background bleeds through).
    - Legacy / fallback GDI rendering paths use the AND mask + XOR mask: AND mask = all 0s means "every pixel is opaque, use the XOR color"; XOR color is (0,0,0). Result: solid 16x16 black square.
    - Windows 11 tray rendering on this user's system clearly hits the AND/XOR path (or the alpha-channel path treats fully-transparent + zero-RGB as the same as opaque-black for icon hosts that pre-multiply), producing the solid black box.

    Why the user perceives icon.ico "renders fine" as the app icon: in `cargo tauri dev` the dev window chrome uses the WebView2 / Chromium default icon, NOT the bundled icon.ico — it's only embedded into the EXE on `cargo tauri build` (release). Dev windows do not actually display icon.ico, so the user is not comparing apples to apples. icon.ico is equally broken; it's just not visible in dev.

- timestamp: 2026-05-09
  checked: git log --follow src-tauri/icons/icon.ico
  found: |
    Only commit: 452d29b (2026-05-08, Phase 1 Plan 01). Commit message explicitly states:
    "[Rule 3 - Blocking] generated minimal multi-layer transparent placeholder icon.ico
    (16/24/32/48/64/256, 32bpp) to satisfy tauri-build's Windows resource embedding step;
    replaced when real artwork lands"
  implication: |
    icon.ico was DESIGNED to be a fully-transparent placeholder, intended to be swapped before any user-visible runtime use. 04-02 copied it to tray.ico, and 04-07 (which was supposed to replace it with real artwork) ended up SFX-only. The icon was never replaced. This is a missed handoff, not a code bug.

- timestamp: 2026-05-09
  checked: src-tauri/tauri.conf.json + src-tauri/build.rs + src-tauri/gen
  found: |
    bundle.icon = ["icons/icon.ico"] only configures bundle artifacts (NSIS installer, etc.).
    No .rc file in src-tauri/gen — Tauri 2's tauri-build embeds the Win32 resource ICO into the release EXE only when bundling, not during `cargo tauri dev`.
  implication: |
    Confirms why dev-mode app windows look "fine" — they aren't using icon.ico at all. The black-box only appears for tray because tray uses Image::from_bytes(include_bytes!("../icons/tray.ico")) which loads the file directly regardless of dev/release mode.

## Resolution

root_cause:
  defect_1: |
    Extra "Hallmark" greyed item is hand-coded in src-tauri/src/tray.rs:62 (created)
    and :77 (placed as first menu item). It is NOT auto-injected by Tauri.

    Behind it: a spec contradiction — 04-CONTEXT.md D-01 (lines 28-38) locks the
    menu WITH a Hallmark header, while 04-UAT.md test 2 expected (line 21) and the
    user's reported expectation specify the menu WITHOUT a header. tray.rs faithfully
    implements the 04-CONTEXT spec.

  defect_2: |
    src-tauri/icons/tray.ico is a fully-transparent placeholder ICO containing zero
    glyph pixels at every resolution (alpha=0 across all 6 frames: 16/24/32/48/64/256).
    Created in commit 452d29b as a build-system-satisfying placeholder explicitly
    marked "replaced when real artwork lands". Copied byte-for-byte to tray.ico in
    Phase 04-02. The intended replacement in Phase 04-07 ended up SFX-only — the
    cosmetic glyph swap never landed.

    Windows tray legacy AND/XOR rendering interprets the all-zero AND mask + zero-RGB
    XOR pixels as a solid opaque black 16x16 square, hence the visible black box.

    The same icon.ico is NOT actually rendered by `cargo tauri dev` window chrome
    (which uses the WebView2 default), so the user perceives it as "fine" — but the
    icon would be equally broken if it were displayed. tray.ico is the only place
    the bytes are loaded at runtime in dev mode (via include_bytes!).

fix:
  defect_1: |
    Recommended (matches user expectation + UAT): remove header.
    src-tauri/src/tray.rs:
      - Delete line 62: `let header = MenuItem::with_id(app, "header", "Hallmark", false, None::<&str>)?;`
      - Delete lines 77 and 78 entries from .items(): `&header,` and the orphan `&sep1,` (sep1 only made sense after the header)
      - Delete line 71: `let sep1 = PredefinedMenuItem::separator(app)?;`
      - Delete lines 151-153 (the "header" branch in handle_menu_event)
      - Update doc comment lines 3-14 to remove Hallmark/sep1 from the menu structure.
      - Update 04-CONTEXT.md D-01 (lines 28-38) to drop the Hallmark header to keep
        spec and code in sync, OR add a 04-UAT.md amendment if the spec decision
        flips the other way.

  defect_2: |
    Replace src-tauri/icons/tray.ico (and ideally icon.ico) with real artwork
    containing actual non-transparent pixels at all frame sizes. Critical sizes for
    the Windows tray: 16x16 and 32x32. v1 acceptable: a single monochrome glyph
    (e.g. white stylized 'H' on transparent background, anti-aliased) generated via
    `magick convert glyph.png -define icon:auto-resize=256,128,64,48,32,24,16 tray.ico`
    or a designed multi-resolution ICO. The exact glyph is a creative choice; the
    constraint is "any non-zero alpha pixel in 16x16/32x32" — anything beats the
    current empty file.

verification: pending — diagnose-only mode (find_root_cause_only). User decides on
  spec direction for defect 1; user generates artwork for defect 2.

files_changed: []
