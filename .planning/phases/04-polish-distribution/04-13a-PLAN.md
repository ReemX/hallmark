---
phase: 04-polish-distribution
plan: 13a
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/tray.rs
  - .planning/phases/04-polish-distribution/04-CONTEXT.md
autonomous: true
gap_closure: true
requirements:
  - POL-01
  - POL-02
tags:
  - gap-closure
  - tray-menu
  - phase4-polish

must_haves:
  truths:
    - "Tray right-click menu shows EXACTLY (in order): Show companion, Fire test popup, Settings…, ☑ Start with Windows, Quit — with separators between functional groups but NO 'Hallmark' header item (UAT test 2 expectation, root cause #1)"
    - "Doc comment in tray.rs:3-14 reflects the new locked layout (no 'Hallmark' header)"
    - "04-CONTEXT.md D-01 (lines 28-38) is amended to drop the Hallmark header + leading separator; a `[SUPERSEDED 2026-05-09 user pick during UAT]` annotation records the change"
    - "Tray menu still functions: left-click on tray icon = Show companion (D-02); right-click → menu (D-01 amended); 'Start with Windows' check-mark reflects live HKCU registry state (D-09)"
  artifacts:
    - path: "src-tauri/src/tray.rs"
      provides: "build_menu without the Hallmark header item; handle_menu_event without the dead 'header' arm; doc comment lines 3-14 rewritten"
      contains: "Show companion"
    - path: ".planning/phases/04-polish-distribution/04-CONTEXT.md"
      provides: "D-01 lines 28-38 amended; SUPERSEDED annotation recording the 2026-05-09 user decision"
      contains: "SUPERSEDED"
  key_links:
    - from: "src-tauri/src/tray.rs::build_menu"
      to: "MenuBuilder.items() — items array"
      via: "header + sep1 entries removed; new array starts with show + test"
      pattern: "Show companion"
---

<objective>
Fix UAT test 2 root cause #1 (Hallmark header in tray menu) — the autonomous
half of the original 04-13 plan. Split out from 04-13 (revision 1, 2026-05-10,
M-1) so the artwork checkpoint in 04-13b does not block downstream wave-2+
plans (04-09 etc.) waiting on this code change.

Per the diagnosis (`.planning/debug/tray-menu-extra-header-and-black-icon.md`):

**Defect 1 — Hallmark header item is hand-coded in tray.rs.** Spec
contradiction: 04-CONTEXT.md D-01 (lines 28-38) currently LOCKS the menu
layout WITH a Hallmark header at top, but UAT test 2 expectation and the
user's reading both say no header. UAT.md gap entry already records the user
decision: "DROP the Hallmark header" (2026-05-09). This plan implements it:
delete the relevant lines from tray.rs AND amend 04-CONTEXT.md D-01 so spec
and code stay in sync.

The artwork half of the original 04-13 (defect 2 — black-square tray icon)
moved to 04-13b along with its blocking human-action checkpoint. 04-13b is
also wave 1 but has `autonomous: false`; the orchestrator runs autonomous
wave-1 plans concurrently and pauses ONLY at the 04-13b checkpoint, so this
plan's exit does not block the rest of wave 1 / wave 2.

Out of scope:
- All artwork work (tray.ico / icon.ico real glyph) — owned by 04-13b.
- Tray icon dark/light theme variants.
- Any other D-01 layout change beyond the header drop.

Output: tray.rs trimmed (5 deletions, 1 doc-comment update). 04-CONTEXT.md
D-01 amended with a clear supersession annotation.
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

@src-tauri/src/tray.rs

<interfaces>
Existing tray.rs::build_menu (lines 60-86):

```rust
fn build_menu(
    app: &AppHandle,
    autostart_on: bool,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    // D-01 LOCKED order:
    //   Hallmark / sep / Show companion / Fire test popup / sep /
    //   Settings… / ☑ Start with Windows / sep / Quit
    let header = MenuItem::with_id(app, "header", "Hallmark", false, None::<&str>)?;          // DELETE
    let show = MenuItem::with_id(app, "show_companion", "Show companion", true, None::<&str>)?;
    let test = MenuItem::with_id(app, "fire_test", "Fire test popup", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
    let autostart = CheckMenuItemBuilder::with_id("toggle_autostart", "Start with Windows")
        .checked(autostart_on)
        .enabled(true)
        .build(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;       // DELETE (only meaningful with header)
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;

    MenuBuilder::new(app)
        .items(&[
            &header,        // DELETE
            &sep1,          // DELETE
            &show,
            &test,
            &sep2,
            &settings,
            &autostart,
            &sep3,
            &quit,
        ])
        .build()
}
```

Existing tray.rs::handle_menu_event "header" arm (lines 151-153):

```rust
"header" => {
    // Non-clickable header row — no-op (disabled in menu but event may still fire).
}
```

Doc comment at the top (lines 3-14) — currently lists Hallmark in the menu spec.

04-CONTEXT.md D-01 block (lines 28-38):

```markdown
- **D-01 Tray menu structure (locked):**
  ```
  Hallmark
  ─────────────
  Show companion
  Fire test popup
  ─────────────
  Settings…
  ☑ Start with Windows
  ─────────────
  Quit
  ```
  Inline checkable "Start with Windows" item. ...
```

UAT.md gap entry (test 2):

```yaml
decision: "Drop header (user pick during UAT 2026-05-09); supersedes original D-01 lock"
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: tray.rs — remove Hallmark header item, leading separator, dead handler arm; update doc comment</name>
  <files>src-tauri/src/tray.rs</files>
  <read_first>
    src-tauri/src/tray.rs (entire — 203 lines; identify the exact lines to delete)
  </read_first>
  <action>
    Edit `src-tauri/src/tray.rs` with 5 surgical deletions and one doc-comment
    rewrite:

    **Deletion 1: line 62** — the header MenuItem creation:

    ```rust
    let header = MenuItem::with_id(app, "header", "Hallmark", false, None::<&str>)?;
    ```

    Remove this line entirely.

    **Deletion 2: line 71** — the leading separator (only meaningful when
    placed after the header):

    ```rust
    let sep1 = PredefinedMenuItem::separator(app)?;
    ```

    Remove this line entirely. After deletion, only `sep2` and `sep3` remain
    in the function — used between (test ↔ settings) and (autostart ↔ quit)
    respectively.

    **Deletion 3 + 4: `&header,` and `&sep1,` in the items array** (currently
    lines 77-78). The MenuBuilder array becomes:

    ```rust
    MenuBuilder::new(app)
        .items(&[
            &show,
            &test,
            &sep2,
            &settings,
            &autostart,
            &sep3,
            &quit,
        ])
        .build()
    ```

    The new menu layout:

    ```
    Show companion
    Fire test popup
    ─────────────
    Settings…
    ☑ Start with Windows
    ─────────────
    Quit
    ```

    **Deletion 5: the dead "header" arm in `handle_menu_event`** (lines 151-153):

    ```rust
    "header" => {
        // Non-clickable header row — no-op (disabled in menu but event may still fire).
    }
    ```

    Remove these 3 lines entirely. The remaining match arms (show_companion,
    fire_test, open_settings, toggle_autostart, quit, fall-through `other`)
    handle every menu ID the new build_menu emits.

    **Doc-comment rewrite: lines 3-14.** Replace the current ASCII art:

    ```rust
    //! ## Menu structure (D-01, locked)
    //! ```text
    //! Hallmark            ← non-clickable header
    //! ─────────────────
    //! Show companion
    //! Fire test popup
    //! ─────────────────
    //! Settings…
    //! ☑ Start with Windows
    //! ─────────────────
    //! Quit
    //! ```
    ```

    With the amended D-01 layout:

    ```rust
    //! ## Menu structure (D-01, amended 2026-05-09 — gap-closure 04-13a)
    //! ```text
    //! Show companion
    //! Fire test popup
    //! ─────────────────
    //! Settings…
    //! ☑ Start with Windows
    //! ─────────────────
    //! Quit
    //! ```
    //!
    //! Original D-01 (Phase 4 plan-phase) included a non-clickable
    //! "Hallmark" header at the top. UAT test 2 (2026-05-09) flagged it
    //! as inconsistent with Discord/Slack/Steam tray-utility convention;
    //! user picked the no-header layout above. CONTEXT.md D-01 amended
    //! with a SUPERSEDED annotation; this code follows the amended spec.
    ```

    Keep all other doc-comment lines (D-02 left-click, D-03 quit, D-09
    autostart) untouched.

    **Build + test the change:**

    ```sh
    cd C:/Users/reema/Documents/Programming/achievements/src-tauri
    cargo build --lib
    cargo test --lib tray
    ```

    Expected: clean build (the dead match arm + unused `let header / sep1`
    bindings are gone, no compile warnings). If any "header" string remains
    in the file, `grep -n` will surface it.

    Verify the menu structure invariants by inspection:

    ```sh
    grep -nv '^//' src-tauri/src/tray.rs | grep -c '"header"'
    ```

    Expected: 0 (no remaining references to the "header" menu ID outside
    comments).
  </action>
  <verify>
    <automated>cd C:/Users/reema/Documents/Programming/achievements/src-tauri &amp;&amp; ! grep -E 'let header|let sep1' src/tray.rs &amp;&amp; ! grep -E '"header"' src/tray.rs &amp;&amp; ! grep -E '&amp;header,|&amp;sep1,' src/tray.rs &amp;&amp; cargo build --lib 2&gt;&amp;1 | tail -5</automated>
  </verify>
  <done>
    tray.rs has no remaining `let header`, `let sep1`, `"header"` ID, or `&header,` / `&sep1,` array entries. Doc comment reflects the amended D-01 layout. `cargo build --lib` succeeds with no warnings.
  </done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: 04-CONTEXT.md — amend D-01 to drop the Hallmark header + leading separator; record SUPERSEDED annotation</name>
  <files>.planning/phases/04-polish-distribution/04-CONTEXT.md</files>
  <read_first>
    .planning/phases/04-polish-distribution/04-CONTEXT.md (lines 26-39 — the D-01 block)
  </read_first>
  <action>
    Edit `.planning/phases/04-polish-distribution/04-CONTEXT.md`. Find the
    D-01 block at lines 27-38:

    ```markdown
    - **D-01 Tray menu structure (locked):**
      ```
      Hallmark
      ─────────────
      Show companion
      Fire test popup
      ─────────────
      Settings…
      ☑ Start with Windows
      ─────────────
      Quit
      ```
      Inline checkable "Start with Windows" item. "Settings…" opens a separate Settings window. Tray is the primary surface for both POL-01 + POL-02.
    ```

    Replace with:

    ```markdown
    - **D-01 Tray menu structure (locked, amended 2026-05-09 — see SUPERSEDED note):**
      ```
      Show companion
      Fire test popup
      ─────────────
      Settings…
      ☑ Start with Windows
      ─────────────
      Quit
      ```
      Inline checkable "Start with Windows" item. "Settings…" opens a separate Settings window. Tray is the primary surface for both POL-01 + POL-02.

      **[SUPERSEDED 2026-05-09 — gap closure 04-13a]** Original D-01 included a
      non-clickable "Hallmark" header item at the top with a separator below
      it. Phase 4 UAT test 2 (2026-05-09) flagged the header as inconsistent
      with the Discord/Slack/Steam tray-utility convention (none of those
      ship a header item). User picked the no-header layout above; the
      `tooltip("Hallmark")` on the tray icon already provides app
      identification on hover. tray.rs implements the amended layout; the
      diagnosis ([.planning/debug/tray-menu-extra-header-and-black-icon.md](../../debug/tray-menu-extra-header-and-black-icon.md))
      records the spec contradiction and the resolution.
    ```

    No other lines in 04-CONTEXT.md are touched. Specifically, do NOT touch:
    - D-02 (tray icon left-click) — stays
    - D-03 (quit semantics) — stays
    - Any other D-N decision

    Verify the amendment landed:

    ```sh
    grep -A2 "D-01 Tray menu structure" .planning/phases/04-polish-distribution/04-CONTEXT.md
    grep -B1 -A1 "SUPERSEDED 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md
    ```

    Expected:
    - First grep returns the amended title line + the new menu ASCII art.
    - Second grep returns the SUPERSEDED annotation block.

    Note that this is a planning-document edit, not a source-code change.
    No build step needed.
  </action>
  <verify>
    <automated>cd C:/Users/reema/Documents/Programming/achievements &amp;&amp; grep -q "amended 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md &amp;&amp; grep -q "SUPERSEDED 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md &amp;&amp; ! grep -A1 "D-01 Tray menu structure" .planning/phases/04-polish-distribution/04-CONTEXT.md | head -3 | grep -E "^[[:space:]]*Hallmark$"</automated>
  </verify>
  <done>
    04-CONTEXT.md D-01 ASCII art no longer contains a `Hallmark` line at the top of the menu listing. SUPERSEDED annotation block present below the new ASCII art with a 2026-05-09 timestamp and a link to the debug session.
  </done>
</task>

<task type="auto" tdd="false">
  <name>Task 3: cargo build --workspace — confirm tray.rs deletions compose cleanly</name>
  <files></files>
  <read_first>
    (no files — runtime check only)
  </read_first>
  <action>
    Run a workspace build to confirm Tasks 1 + 2 leave the tree in a buildable
    state. The 04-CONTEXT.md edit is documentation-only and does not affect
    compile; this verification primarily checks Task 1's tray.rs deletions.

    ```sh
    cd C:/Users/reema/Documents/Programming/achievements/src-tauri
    cargo build --workspace
    cargo test --lib
    ```

    Expected: clean workspace build with no warnings about unused bindings
    (header / sep1 are gone), no warnings about unreachable match arms
    (the "header" arm is gone), and all preserved tests pass.

    If any new warnings surface that mention `header` or `sep1`, return to
    Task 1 — a deletion was missed somewhere.
  </action>
  <verify>
    <automated>cd C:/Users/reema/Documents/Programming/achievements/src-tauri &amp;&amp; cargo build --workspace 2&gt;&amp;1 | tail -10 &amp;&amp; cargo test --lib 2&gt;&amp;1 | tail -10</automated>
  </verify>
  <done>
    Workspace build clean. `cargo test --lib` passes. tray.rs has no remaining `header` / `sep1` references (Tasks 1 + 2 verified together).
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Build-bake | tray menu definition is compiled into the binary at compile time — no runtime decision surface |
| Spec/code drift | The CONTEXT.md amendment + tray.rs change ship in the same plan to prevent the spec from being out of sync with code |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-04G-25 | E (Elevation of Privilege) | The amended D-01 spec drift between code and CONTEXT.md | mitigate | Task 2 amends 04-CONTEXT.md to match Task 1's tray.rs change in the same plan. Both edits ship in the same commit (Task 3 verifies both before claiming done). Future Phase 4 contributors reading either source see the same locked menu structure. |
| T-04G-25b | T (Tampering) | A future tray.rs edit re-introduces a "header" MenuItem without the corresponding CONTEXT.md amendment | mitigate | The doc comment on tray.rs:3-14 carries a paragraph linking back to the SUPERSEDED note. Anyone editing tray.rs who tries to re-add a header has to reckon with the explicit "do not re-add the header" history in the file itself. |
</threat_model>

<verification>
- `cargo build --workspace` succeeds.
- `cargo test --lib` passes (all crates).
- `grep -E '"header"|let header|&header,' src-tauri/src/tray.rs` returns 0 hits.
- `grep "amended 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md` returns 1 hit.
- `grep "SUPERSEDED 2026-05-09" .planning/phases/04-polish-distribution/04-CONTEXT.md` returns 1 hit.
- Manual UAT re-verification (deferred to phase-level UAT re-run): tray right-click menu has no Hallmark header, all 5 menu items work, left-click shows companion. The black-square tray icon defect is owned by 04-13b — that issue persists until 04-13b's artwork checkpoint completes.
</verification>

<success_criteria>
- UAT test 2 root cause #1 closed: tray menu matches the amended D-01 layout (no Hallmark header).
- 04-CONTEXT.md D-01 amended with a clear SUPERSEDED annotation — spec and code stay in sync.
- The Hallmark `tooltip("Hallmark")` on the tray icon (already in tray.rs build_tray) provides app identification on hover, matching the diagnosis recommendation.
- This plan is autonomous and unblocks downstream waves; root cause #2 (black-square tray icon) is owned by 04-13b which can run in parallel without blocking other plans.
</success_criteria>

<output>
After completion, create `.planning/phases/04-polish-distribution/04-13a-SUMMARY.md` capturing:
- Code deletions: 5 in tray.rs (lines 62, 71, two array entries, dead handler arm) + doc-comment rewrite
- 04-CONTEXT.md D-01 amendment (with SUPERSEDED annotation linking to .planning/debug/tray-menu-extra-header-and-black-icon.md)
- UAT items closed: test 2 root cause #1 (Hallmark header)
- Note: root cause #2 (black-square tray icon) is owned by 04-13b
</output>

## Revision Log

| Iteration | Date | Finding | Change |
|-----------|------|---------|--------|
| 1 | 2026-05-10 | M-1 | Plan created by splitting the original 04-13. This plan retains the autonomous tray.rs deletions + 04-CONTEXT.md D-01 amendment so the wave-1 autonomous critical path is not blocked by the artwork human checkpoint (now in 04-13b). |
