---
phase: 04-polish-distribution
plan: 13b
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/icons/tray.ico
  - src-tauri/icons/icon.ico
autonomous: false
gap_closure: true
requirements:
  - POL-01
  - POL-02
tags:
  - gap-closure
  - tray-icon
  - artwork
  - phase4-polish

must_haves:
  truths:
    - "src-tauri/icons/tray.ico is a real multi-resolution ICO with non-zero alpha pixels at minimum 16×16 + 32×32 — replaces the fully-transparent placeholder from Phase 1 commit 452d29b"
    - "src-tauri/icons/icon.ico (currently byte-identical placeholder) is also replaced with real artwork so the NSIS-installed binary's window/taskbar/installer icon is recognizable"
    - "Tray icon in Windows 11 notification area renders as a recognizable Hallmark glyph, NOT a solid black square"
  artifacts:
    - path: "src-tauri/icons/tray.ico"
      provides: "Real multi-resolution ICO (≥16×16 + ≥32×32 + at least one larger frame) with visible non-zero-alpha glyph pixels"
      contains: "(binary asset — verified by ICO header parse)"
    - path: "src-tauri/icons/icon.ico"
      provides: "Same artwork (or designer-recommended variant) for the NSIS-bundled window/taskbar/installer icon"
      contains: "(binary asset — verified by ICO header parse)"
  key_links:
    - from: "src-tauri/src/tray.rs::Image::from_bytes"
      to: "src-tauri/icons/tray.ico"
      via: "include_bytes!(\"../icons/tray.ico\")"
      pattern: "include_bytes.*tray\\.ico"
---

<objective>
Fix UAT test 2 root cause #2 (black-square tray icon) — the human-checkpoint
half of the original 04-13 plan. Split out from 04-13 (revision 1, 2026-05-10,
M-1) so the artwork checkpoint does not block downstream wave-2+ plans
(04-09 etc.).

This plan is `autonomous: false` because the artwork must be USER-SUPPLIED.
The project lacks a designer; Hallmark's signature style is locked per
CONTEXT.md. v1 acceptance is "any non-zero alpha pixels in 16×16 and 32×32
frames" — a monochrome stylized 'H' or trophy glyph satisfies the threshold;
polish is a v1.1 concern.

Per the diagnosis (`.planning/debug/tray-menu-extra-header-and-black-icon.md`):

**Defect 2 — tray.ico (and icon.ico) are fully-transparent placeholder ICOs.**
Phase 1 commit 452d29b created a multi-frame ICO (16/24/32/48/64/256) with
ALL pixels alpha=0 — explicitly marked "replaced when real artwork lands"
in the commit message. Phase 4 Plan 04-07 was supposed to land real artwork
but pivoted to SFX-only. The cosmetic glyph swap never happened. Windows 11
tray icon host (legacy GDI AND/XOR rendering path) interprets the all-zero
mask as solid black, hence the black square.

**M-1 wave-blocking note:** No downstream plan declares
`depends_on: ["04-13b"]`. This plan is wave 1 alongside 04-13a, but its
human-action checkpoint pauses ONLY this plan; the rest of wave 1 (04-08,
04-13a) can complete autonomously, and wave-2+ plans (04-09, 04-10, 04-11,
04-12) can begin as soon as their declared dependencies finish — they do
NOT wait for 04-13b's checkpoint. The orchestrator should be configured to
run autonomous plans in parallel and surface checkpoints inline without
gating other independent waves.

Out of scope:
- Tray icon dark/light theme variants — UI-SPEC § Tray Menu mentions the
  option but doesn't lock either choice; v1 ships single-icon for both themes.
- Header / menu deletions — owned by 04-13a.

Output: Two ICO files replaced (via the user's checkpoint) plus an automated
Python alpha verification step plus a tray-icon visual UAT.
</objective>

<execution_context>
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/workflows/execute-plan.md
@C:/Users/reema/Documents/Programming/achievements/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/04-polish-distribution/04-CONTEXT.md
@.planning/phases/04-polish-distribution/04-UI-SPEC.md
@.planning/phases/04-polish-distribution/04-UAT.md
@.planning/debug/tray-menu-extra-header-and-black-icon.md

@src-tauri/icons/tray.ico
@src-tauri/icons/icon.ico

<interfaces>
Diagnosis-suggested artwork generation (one-liner via ImageMagick):

```sh
magick convert glyph.png -define icon:auto-resize=256,128,64,48,32,24,16 tray.ico
```

V1 acceptance: ANY non-zero alpha pixel in the 16×16 + 32×32 frames. The
exact glyph design is the user's pick — diagnosis suggests "monochrome
stylized 'H' or trophy glyph".
</interfaces>
</context>

<tasks>

<task type="checkpoint:human-action" gate="blocking">
  <name>Task 1: HUMAN — drop in real artwork for src-tauri/icons/tray.ico AND src-tauri/icons/icon.ico</name>
  <what-built>
    The current `src-tauri/icons/tray.ico` and `src-tauri/icons/icon.ico`
    are byte-identical placeholders with all pixels alpha=0. Result: solid
    black square in the Windows 11 tray. The remaining defect (test 2 root
    cause #2) requires user-supplied artwork — the project lacks a designer
    and Hallmark's signature style is locked per CONTEXT.md.

    Plan 04-13a closed root cause #1 (Hallmark header drop) autonomously;
    this plan owns root cause #2 (black-square icon).
  </what-built>
  <how-to-verify>
    YOU MUST DO THIS — no automation can substitute. The icon files are
    binary assets that you own creatively.

    **Step 1: Design or source the glyph.**

    UI-SPEC § Tray Menu suggests "a simple trophy/medal outline glyph,
    rendered white on dark taskbar or black on light taskbar" — but any
    monochrome glyph satisfies v1 acceptance.

    Candidate sources:
    - Hand-drawn in Figma/Krita/Inkscape (export as PNG at 256×256, white
      on transparent background).
    - Font glyph: pick a thematic Unicode codepoint (🏆, 🎖, ⭐, or just a
      stylized 'H'), render in a monospace font at 256×256, export as PNG.
    - CC0 icon repo: https://lucide.dev/, https://feathericons.com/, or
      https://heroicons.com/ — pick a relevant glyph (trophy, award, star),
      render as PNG.

    Constraint: white-on-transparent (or light-grey-on-transparent) so the
    Windows tray's auto-invert behavior produces visible glyphs on both
    light and dark taskbars. v1 acceptance: ANY non-zero alpha pixels in
    16×16 and 32×32 frames.

    **Step 2: Generate the multi-resolution ICO.**

    With ImageMagick (`magick.exe`) installed:

    ```sh
    magick convert glyph.png -define icon:auto-resize=256,128,64,48,32,24,16 src-tauri/icons/tray.ico
    magick convert glyph.png -define icon:auto-resize=256,128,64,48,32,24,16 src-tauri/icons/icon.ico
    ```

    (You can use the same source PNG for both files. Diagnosis recommends
    they be the same since `tray.ico` is byte-identical to `icon.ico` today
    by intent.)

    Without ImageMagick: use any free ICO converter that supports multi-
    resolution output, e.g. https://convertio.co/png-ico/ (verify CC0 / no
    EULA conflicts before uploading) or `pip install Pillow` + a 5-line
    Python script:

    ```python
    from PIL import Image
    img = Image.open("glyph.png")
    img.save("src-tauri/icons/tray.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
    img.save("src-tauri/icons/icon.ico", sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
    ```

    **Step 3: Verify the new ICO files have non-zero alpha pixels (manual peek).**

    Open `tray.ico` in Windows Explorer's icon preview at 256×256 — must
    show your glyph, not a transparent / black rectangle. (Task 2 runs an
    automated alpha check after this checkpoint resumes.)

    **Step 4: Resume signal.**

    Type `approved` once both ICO files are real artwork (not all-transparent)
    and the tray icon in Windows Explorer's preview shows your glyph.

    If the artwork is "good enough for v1" but you want to iterate later,
    type `approved-v1-iterate` to record the deferral. v1.1 can ship a
    polished version without re-running this plan.
  </how-to-verify>
  <resume-signal>
    Type `approved` once both `src-tauri/icons/tray.ico` and
    `src-tauri/icons/icon.ico` are real artwork and previews show a
    visible glyph (not a solid black or transparent rectangle). Type
    `approved-v1-iterate` if you want to ship v1 with a "good enough"
    glyph and revisit the artwork in v1.1. (Both signals proceed to
    Tasks 2 + 3 — the difference is a documentation note in the SUMMARY.)
  </resume-signal>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Python alpha verification — confirm non-zero alpha at 16×16 + 32×32 frames in both ICO files</name>
  <files></files>
  <read_first>
    src-tauri/icons/tray.ico (binary inspection only),
    src-tauri/icons/icon.ico (binary inspection only)
  </read_first>
  <action>
    After Task 1 resumes, run an automated alpha check to confirm the
    user-supplied artwork actually meets v1 acceptance.

    **Step A: Confirm both ICO files are non-trivially populated.**

    ```sh
    cd C:/Users/reema/Documents/Programming/achievements
    ls -la src-tauri/icons/tray.ico src-tauri/icons/icon.ico
    ```

    Expected: each file size > 1 KB (multi-resolution ICO with real glyph
    pixels is typically 50-300 KB). Note: file size alone does not
    differentiate — the Phase 1 placeholder was 304886 bytes but every
    pixel was alpha=0. The Python check below is the load-bearing
    verification.

    **Step B: Run the Python alpha check** (paste this entire block into
    a Python REPL or save as a `.py` file and run it):

    ```python
    from PIL import Image
    failures = []
    for path in ["src-tauri/icons/tray.ico", "src-tauri/icons/icon.ico"]:
        img = Image.open(path)
        n_frames = getattr(img, "n_frames", 1)
        sizes_seen = set()
        for frame_idx in range(n_frames):
            if hasattr(img, "seek"):
                img.seek(frame_idx)
            sizes_seen.add(img.size)
            data = list(img.convert("RGBA").getdata())
            non_zero = sum(1 for px in data if px[3] != 0)
            if img.size == (16, 16) and non_zero == 0:
                failures.append(f"{path} 16x16 has zero alpha — defect 2 not closed")
            if img.size == (32, 32) and non_zero == 0:
                failures.append(f"{path} 32x32 has zero alpha — defect 2 not closed")
        if (16, 16) not in sizes_seen:
            failures.append(f"{path} missing 16x16 frame")
        if (32, 32) not in sizes_seen:
            failures.append(f"{path} missing 32x32 frame")
    if failures:
        raise SystemExit("\n".join(failures))
    print("ICO artwork acceptance: PASS")
    ```

    Expected output: `ICO artwork acceptance: PASS`. If `failures` surface,
    return to Task 1 — the artwork did not satisfy v1 acceptance.

    **Step C: Workspace build.**

    The ICO change does not affect Rust compilation directly (the bytes
    are `include_bytes!`d so any valid ICO file passes), but the build
    confirms tray.rs (after 04-13a) still compiles cleanly with the new
    ICO bytes baked in.

    ```sh
    cd C:/Users/reema/Documents/Programming/achievements/src-tauri
    cargo build --workspace
    ```
  </action>
  <verify>
    <automated>cd C:/Users/reema/Documents/Programming/achievements &amp;&amp; python -c "from PIL import Image; failures=[]; \
[ (lambda p: [(failures.append(f'{p} {img.size} alpha=0') if img.size in [(16,16),(32,32)] and sum(1 for px in img.convert('RGBA').getdata() if px[3]!=0)==0 else None) for img in [Image.open(p)] for f in [getattr(Image.open(p),'n_frames',1)] for i in range(f) if (img.seek(i) or True) ])(p) for p in ['src-tauri/icons/tray.ico','src-tauri/icons/icon.ico'] ]; \
print('FAIL: '+';'.join(failures)) if failures else print('ICO artwork acceptance: PASS')" &amp;&amp; cd src-tauri &amp;&amp; cargo build --workspace 2&gt;&amp;1 | tail -5</automated>
  </verify>
  <done>
    Python alpha check prints "ICO artwork acceptance: PASS". Both ICO files have non-zero alpha at the 16×16 and 32×32 frames. `cargo build --workspace` succeeds.
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: HUMAN — visual UAT of the new tray icon in Windows 11</name>
  <what-built>
    Tasks 1 + 2 replaced the all-transparent placeholder ICOs with real
    artwork and verified non-zero alpha at the v1 minimum frame sizes.
    This checkpoint confirms the visual outcome in the live Windows 11
    tray.
  </what-built>
  <how-to-verify>
    1. `cargo tauri dev`
    2. Look at the Windows 11 tray icon overflow area: your glyph should be
       visible. NOT a solid black square. NOT a transparent gap.
    3. Right-click → menu shows the amended D-01 layout (no Hallmark header,
       per 04-13a). All 5 menu items work, autostart toggle reflects HKCU.
    4. Left-click tray → companion shows (D-02 unchanged).
    5. Hover the tray icon — tooltip "Hallmark" appears (build_tray sets this).

    If the icon still renders black or invisible, the ICO file may be
    missing a 16×16 or 32×32 frame even though Task 2 passed (e.g. if a
    converter only emitted a 256×256 frame). Re-run Task 1 with explicit
    multi-resolution generation.

    Acceptance for v1: a recognizable glyph (does not have to be polished).
    Polish is a v1.1 slot.
  </how-to-verify>
  <resume-signal>
    Type `approved` if the tray icon is visibly NOT a solid black square
    and the right-click menu matches the amended D-01 layout. Type
    `issue: <description>` if anything fails — the orchestrator surfaces
    the issue back through gap-closure planning.
  </resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Build-bake | tray.ico bytes are `include_bytes!`d into the binary at compile time — no runtime path |
| User-supplied artwork | The ICO files are committed to the repo; redistribution attaches Hallmark's brand identity |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04G-22 | T (Tampering) | A malicious ICO file with embedded payload (e.g. CVE-2017-7493) | accept | The user generates the ICO from a trusted PNG source via ImageMagick or Pillow (both audited). No download path from a third-party ICO is recommended; if the user does download, they validate via the alpha check + size check. The risk is symmetric to any other binary asset in the repo. |
| T-04G-23 | I (Information Disclosure) | The chosen glyph reveals branding direction prematurely | accept | Hallmark's brand identity is intentionally surfaced — the project README and GitHub Release pages both show the icon. No information advantage to keeping it hidden. |
| T-04G-24 | T | A future ICO update breaks the include_bytes! path resolution | accept | Tauri 2's `Image::from_bytes` validates the ICO header at startup and panics with a clear error if invalid. tray::build_tray uses `tauri::Result<()>` and is best-effort: failure logs `tracing::warn!` and continues without a tray. The detection pipeline is unaffected — graceful degradation. |
</threat_model>

<verification>
- Both ICO files are real artwork (Task 1 user-supplied + Task 2 alpha check confirms non-zero alpha at 16×16 + 32×32).
- Tray icon in Windows 11 renders as a recognizable glyph (Task 3 manual UAT).
- `cargo build --workspace` succeeds (Task 2 step C).
- The tray menu structure (no Hallmark header) is owned by 04-13a; this plan does NOT modify tray.rs.
</verification>

<success_criteria>
- UAT test 2 root cause #2 closed: tray icon renders as a recognizable Hallmark glyph (any non-zero-alpha 16×16 + 32×32 ICO satisfies v1 acceptance).
- icon.ico (NSIS-bundled installer/window/taskbar icon) also replaced — Phase 1 placeholder retired.
- This plan's blocking checkpoints pause ONLY this plan; downstream waves are not gated on the artwork checkpoint because no plan declares depends_on: ["04-13b"].
</success_criteria>

<output>
After completion, create `.planning/phases/04-polish-distribution/04-13b-SUMMARY.md` capturing:
- ICO artwork swap: source / generation method / glyph design (record for future re-generation)
- Note whether human resumed Task 1 with `approved` (artwork shipped clean) or `approved-v1-iterate` (artwork good-enough; v1.1 polish slot)
- Task 3 visual UAT result
- UAT items closed: test 2 root cause #2 (black tray icon)
</output>

## Revision Log

| Iteration | Date | Finding | Change |
|-----------|------|---------|--------|
| 1 | 2026-05-10 | M-1 | Plan created by splitting the original 04-13. This plan owns the human-action artwork checkpoint + Python alpha verification + tray-icon visual UAT. Both checkpoints pause this plan only — no downstream plan declares depends_on: ["04-13b"], so wave-2+ plans (04-09 et al.) are not blocked by the artwork delivery. |
