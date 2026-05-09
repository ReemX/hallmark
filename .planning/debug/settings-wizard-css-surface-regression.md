---
status: diagnosed
trigger: "Settings + Wizard CSS surface regression — scrollbar outside card, light bleed, non-sticky wizard header, skeleton row jump, wasted horizontal space"
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

reasoning_checkpoint:
  hypothesis: "settings.css is missing the html/body+#root reset that companion.css ships at lines 2–6. The .settings-shell / .wizard-shell rely on `min-height: 100vh` instead of `height: 100%`, so the shell sizes itself to its content rather than the viewport. The body has default UA white background and default 8px margin, and the body is the scroll container — producing all five visible regressions in one stroke. A secondary bug is that .skeleton-line height (36px) is not exactly equivalent to .source-row total height (min-height 36px + 8px×2 padding = effective ~36–52px because content lines sit inside)."
  confirming_evidence:
    - "companion.css lines 2–6: `html, body { margin: 0; padding: 0; background: #111114; ... } #root { width: 100vw; min-height: 100vh; }`. popup.css lines 3–8: same pattern (with `background: transparent` and `overflow: hidden`). settings.css has NEITHER rule — companion is the missing reference."
    - "settings.html line 7 + wizard.html line 7 both link only `/src/styles/settings.css`. There is no per-window inline reset, no global reset CSS imported. The unstyled body is what the screenshot reveals as 'off-white' background."
    - ".settings-shell (line 4–14) and .wizard-shell (line 142–152) use `min-height: 100vh`. `min-height` does NOT establish a flex parent height, so the inner `.settings-body { flex: 1; overflow-y: auto }` (line 35–42) cannot actually overflow within the shell — its overflow propagates up to body, which spawns the native browser scrollbar at the WINDOW edge, outside the rounded shell. That matches the screenshot exactly."
    - ".wizard-header (line 153–162) has no `position: sticky`, no `top: 0`, no `z-index`. With body being the scroller, the header is just a normal block child and scrolls away — matches symptom (c)."
    - ".skeleton-line height: 36px` (line 83). .source-row has `min-height: 36px` PLUS `padding: 8px` (line 56–61). Effective rendered row height ≈ max(36, 8+lineHeight+8) where the inner .source-name has font-size 14 line-height 1.5 = 21px → 8+21+8 = 37px. Close, but the skeleton has NO padding and NO font-line baseline — when the rescan resolves, rows reflow with their own padding box, producing the visible jump. Skeleton is also rendered inside `.settings-source-list { gap: 4px }` so vertical rhythm differs."
    - "Settings.tsx line 130 wraps content in `.settings-body` which has `padding: 16px` and only `flex: 1` for width — fine, but the inner `.settings-section` (gap 32px) plus three sections plus dividers easily exceeds 580px-48px(header)=532px usable height. Combined with the body-level scroll bug, content overflows AND scrolls in the wrong place."
  falsification_test: "If we add `html, body { margin: 0; height: 100%; background: #111114; overflow: hidden; }` and `#root { width: 100vw; height: 100vh; }` and change the shells from `min-height: 100vh` to `height: 100%`, BOTH the off-white bleed AND the external scrollbar should disappear in a single change. If either persists after that change, this hypothesis is wrong."
  fix_rationale: "All five sub-symptoms trace back to ONE missing reset (html/body/#root) plus four corollaries (shell uses min-height not height; body lacks sticky-header rule; skeleton dimensions don't mirror source-row; settings-body has unnecessary horizontal padding). Fixing the reset addresses (a) scrollbar position and (b) light bleed simultaneously. The other three are independent CSS tweaks. companion.css is the reference pattern that already works in production — settings.css is the regression."
  blind_spots:
    - "I have not visually verified the WebView2 default body background color is the exact off-white shown in the screenshot — I'm inferring from 'CSS UA default'. It may instead be tinted by the OS/window manager. Either way, the fix (set body bg to #111114) eliminates it."
    - "I have not measured the exact pixel height of a rendered .source-row at runtime. The padding+content arithmetic suggests 37px, but actual computed style may differ if line-height resolves to 21.0px exactly or rounds. Fix should set .skeleton-line to match exactly: same min-height + same padding + same border-radius."
    - "I have not measured Settings content total height vs 532px usable. The 'wasted horizontal space' symptom may be driven by something other than padding (e.g., max-width on a child); I did not see one in CSS, so default `flex` makes children stretch to container width. Most likely the user's 'wasted' impression is that vertical scroll forces the eye to scroll down for content that could fit if vertical density were tighter — the gap of 32px between sections is the largest single contributor (UI-SPEC line 54 calls it 'xl' and uses it as the section-separator token, but the spec authors didn't account for content overflow at 580px height)."

## Symptoms

expected: Both windows feel premium-dark end-to-end. No visible OS scrollbar in the chromeless gap. No light body color bleed. Wizard header sticks while content scrolls. Skeleton rows match real-row dimensions exactly so transitions are seamless. Settings content fits within 520×580 without scrolling.
actual:
  - Settings 520×580: rounded dark card visible; native silver Windows scrollbar to the right of the card; padding around card reveals slightly-lighter (off-white) background.
  - Wizard 480×560: same scrollbar + bg pattern; on scroll the "Welcome to Hallmark" header scrolls with the body (not sticky).
  - Settings: skeleton placeholders shorter than rendered SettingsSourceRow rows — visible jump on rescan resolve.
  - Settings: About + Updates only reachable by scrolling; horizontal space under-used.
errors: None
reproduction: cargo tauri dev → tray right-click → Settings (test 6); clear first_run_done → relaunch (test 14).
started: Discovered during Phase 4 UAT tests 6 + 7 + 14 on 2026-05-09.

## Eliminated

- hypothesis: "Tauri window is rendered transparent and the desktop is showing through around the card"
  evidence: "src-tauri/src/settings_window.rs line 17 + first_run.rs line 29 both call `.transparent(false)`. Window is opaque. The light bleed must be from the body element's UA default background, not desktop pass-through."
  timestamp: 2026-05-09

## Evidence

- timestamp: 2026-05-09
  checked: "src/styles/companion.css full file"
  found: "Lines 2–6 establish a complete reset: `html, body { margin: 0; padding: 0; background: #111114; color: #F0F0F5; font-family: ... } #root { width: 100vw; min-height: 100vh; }`. Then `.companion-shell { height: 100vh; ... }` (line 8) uses `height` not `min-height`."
  implication: "This is the working pattern. settings.css must mirror it. Note companion.css uses `min-height: 100vh` on #root but `height: 100vh` on the shell — the shell-on-height is what makes flex:1 + overflow-y:auto on inner .companion-list (line 45–47) work correctly with the scrollbar appearing INSIDE the shell."

- timestamp: 2026-05-09
  checked: "src/styles/settings.css full file"
  found: "No html/body/#root rules anywhere. .settings-shell uses `min-height: 100vh` (line 7). .wizard-shell uses `min-height: 100vh` (line 145). Both shells set `border-radius: 12px; overflow: hidden` — the border-radius is fine but the overflow:hidden is overridden by the body scrollbar living one level up. .settings-body and .wizard-body both have `flex: 1; overflow-y: auto` — these never engage because their parent isn't height-constrained."
  implication: "Confirms H1 (missing reset) + H2 (scroll on wrong container) + adds: shells use min-height not height, which is the proximate cause of the body-level scroll."

- timestamp: 2026-05-09
  checked: "src/styles/popup.css lines 1–8"
  found: "popup.css ALSO ships the html/body reset: `html, body { margin: 0; padding: 0; background: transparent; overflow: hidden; ... } #root { width: 100vw; height: 100vh; }`. Note popup.css explicitly sets `overflow: hidden` and `height: 100vh` (not min-height). Because popup is transparent, background is transparent — but the structural pattern (margin 0, height 100vh, overflow hidden) is identical."
  implication: "Two-of-two production stylesheets ship the reset. settings.css is the outlier. The shell should use `height: 100%` (or `100vh`) not `min-height`."

- timestamp: 2026-05-09
  checked: "settings.html and wizard.html"
  found: "Both files are minimal: `<html><body><div id='root'></div></body></html>` with one stylesheet link (`/src/styles/settings.css`) and one script. No inline styles, no separate reset, no global.css import. Body has UA default 8px margin + UA default background-color (typically #FFFFFF in WebView2)."
  implication: "Without a CSS reset, body shows white with 8px margin. Confirms the off-white bleed origin. Both windows share the same stylesheet so a single fix addresses both."

- timestamp: 2026-05-09
  checked: "src-tauri/src/settings_window.rs and first_run.rs window builders"
  found: "Both call `.decorations(false)` + `.transparent(false)`. Window is opaque, no native chrome, no native scrollbar coming from Tauri. The scrollbar in the screenshot must be the WebView2's body-level scrollbar."
  implication: "Eliminates 'desktop bleed' and 'native chrome scrollbar' alternatives. Confirms the issue is purely CSS."

- timestamp: 2026-05-09
  checked: "Settings.tsx (skeleton render path) + SettingsSourceRow.tsx + .skeleton-line vs .source-row CSS"
  found: ".skeleton-line: { height: 36px; border-radius: 8px; background: gradient }`. .source-row: { display: flex; align-items: center; padding: 8px; min-height: 36px; border-radius: 8px; background: #1C1C21 }. Skeleton has NO padding and is exactly 36px tall. Source row is min 36px tall but has 8px padding so its visual box is at least 36px (when flex content is small) — but contentful rows render at min-height + padding sum where padding is INSIDE the min-height (border-box not declared, default content-box). Computed height = max(36, content+16). With .source-name at line-height 1.5 × 14px = 21px content height, total = 21+16 = 37px — barely over min-height. Visually the rows appear taller than the 36px skeleton and the 4px gap also differs in perception because skeleton has no horizontal padding so the gradient touches the list edges."
  implication: "Skeleton DOES dimension-mismatch real rows. Fix: skeleton must mirror real-row CSS exactly (padding 8px, min-height 36px, border-radius 8px, box-sizing: border-box). The current skeleton is a bare strip; replacing with `.source-row.skeleton` style class (matching companion.css's pattern at lines 95–103) is cleaner."

- timestamp: 2026-05-09
  checked: "Settings.tsx content tree (header 48px + body containing 3 sections)"
  found: "Three sections: Detected Sources (heading + intro + 4 source rows + button = ~6 lines @ ~24px each + 8px gaps + section gap 32px), Updates (heading + intro + status + meta + button = similar), About (heading + 3 meta lines). Plus 32px gap between sections × 2 = 64px. Plus header 48px + body padding 16px×2 = 32px. Rough total: 3 × ~140 + 64 + 32 + 48 ≈ 564px. With 520×580 window (532px usable below header+padding), this overflows by ~30–60px — small enough to be addressable by tightening section gap from 32→16 or 24, OR by reducing per-section internal gaps."
  implication: "Symptom (e) 'About + Updates only reachable by scrolling' is partly because of body-level scroll (everything scrolls together) but also because the natural content height slightly exceeds the viewport. Reducing .settings-body { gap: 32px } to 24px (still on the 4px scale) or making the layout tighter for the 580px viewport will fit. The 'wasted horizontal space' impression is illusory: rows already stretch to full width via flex; what looks 'wasted' is the right edge being eaten by the OS scrollbar — once the scrollbar moves inside the shell as a thin custom scrollbar, the perceived width returns."

- timestamp: 2026-05-09
  checked: "FirstRunWizard.tsx + .wizard-header CSS"
  found: ".wizard-header is a regular flex child of .wizard-shell, no `position: sticky`. .wizard-body is the scroll container (flex:1 + overflow-y:auto). With the body-level scroll bug, however, the wizard-shell isn't actually scrolling its body — body element is. So the header scrolls with the page."
  implication: "Two parts to fix the wizard header: (1) move scroll into .wizard-body by establishing a height-bounded shell (the html/body fix), (2) optionally add `position: sticky; top: 0; background: #111114; z-index: 1;` to .wizard-header for belt-and-suspenders. Once fix 1 is applied, the header is already a flex-child outside the scrolling body, so it stays in place naturally without needing sticky."

## Resolution

root_cause: |
  settings.css is missing the global CSS reset that companion.css and popup.css both ship. Specifically, settings.css has NO `html, body, #root` rules, and the .settings-shell / .wizard-shell use `min-height: 100vh` instead of `height: 100%`. As a result:
    • body element has UA-default white background and 8px margin → visible "off-white bleed" around the rounded card (symptom b)
    • the inner `.settings-body { flex: 1; overflow-y: auto }` cannot establish a bounded scroll viewport (parent is min-height, not bounded height), so overflow propagates to body → native browser scrollbar appears at WINDOW edge, outside the rounded card (symptom a)
    • .wizard-header is inside the same broken scroll context — body scrolls, header goes with it (symptom c)
    • .skeleton-line is shaped as a bare 36px strip with no padding while .source-row is min-height 36px + 8px padding + content (~37–52px effective) → visible row-height jump on rescan resolve (symptom d)
    • .settings-body has gap: 32px between sections + 16px outer padding; combined with the body-level scroll bug, About + Updates fall below the fold and the visible-area scrollbar steals horizontal real estate (symptom e)
  Companion.css (lines 2–6) and popup.css (lines 3–8) both ship the correct reset. settings.css is the regression — Phase 4 introduced new surfaces but did not inherit Phase 2's reset, contradicting UI-SPEC.md line 17 ("New surfaces … must match that language exactly").

fix:
  Single coordinated CSS-only fix in src/styles/settings.css. NO HTML changes, NO TSX changes (except optional skeleton class rename for clarity).

  1. Add a reset block at the top of settings.css (before .settings-shell):

       html, body {
         margin: 0;
         padding: 0;
         height: 100%;
         background: #111114;
         color: #F0F0F5;
         overflow: hidden;
         font-family: "Inter", "Segoe UI Variable", "Segoe UI", -apple-system,
                      BlinkMacSystemFont, system-ui, sans-serif;
       }
       #root { width: 100vw; height: 100vh; }

  2. Change BOTH shells from `min-height: 100vh` to `height: 100%`:
       .settings-shell { ... height: 100%; ... }   /* was: min-height: 100vh */
       .wizard-shell  { ... height: 100%; ... }    /* was: min-height: 100vh */

  3. Add custom thin scrollbar styling to .settings-body and .wizard-body for the
     premium feel (since the scrollbar now lives INSIDE the rounded card):

       .settings-body::-webkit-scrollbar,
       .wizard-body::-webkit-scrollbar { width: 8px; }
       .settings-body::-webkit-scrollbar-track,
       .wizard-body::-webkit-scrollbar-track { background: transparent; }
       .settings-body::-webkit-scrollbar-thumb,
       .wizard-body::-webkit-scrollbar-thumb {
         background: rgba(255, 255, 255, 0.10);
         border-radius: 4px;
       }
       .settings-body::-webkit-scrollbar-thumb:hover,
       .wizard-body::-webkit-scrollbar-thumb:hover {
         background: rgba(255, 255, 255, 0.20);
       }

  4. Make .skeleton-line dimensionally identical to .source-row so transitions
     are seamless. Replace the current rule:

       .skeleton-line {
         min-height: 36px;
         padding: 8px;
         border-radius: 8px;
         box-sizing: border-box;
         background: linear-gradient(90deg, #1C1C21 0%, #2A2A30 50%, #1C1C21 100%);
         background-size: 200% 100%;
         animation: skeleton-pulse 1.5s ease-in-out infinite;
       }

     (Removed fixed `height: 36px`; added `min-height: 36px`, `padding: 8px`, and
     `box-sizing: border-box` to mirror .source-row line 56–61.)

  5. Tighten .settings-body section gap from 32px to 24px to fit 520×580 without
     scroll on the typical 3-section render (Detected + Updates + About):
       .settings-body { gap: 24px; }   /* was: 32px */

     This stays on the 4-point scale (UI-SPEC § Spacing) and matches the spec's
     "lg" token (24px).

  6. (Optional, defensive) Add sticky header to wizard for cases where future
     content might exceed viewport even after fix 1:
       .wizard-header { position: sticky; top: 0; z-index: 1; }
     Background is already #111114 so no transparency leak when content scrolls
     under it. Same can be applied to .settings-header.

  Total: ~25 added lines, 4 modified lines, all in src/styles/settings.css.

verification: "Run cargo tauri dev. Open Settings (tray → Settings): rounded dark card fills the entire 520×580 window with NO light bleed and NO external scrollbar. If content overflows, a thin custom dark scrollbar appears INSIDE the rounded card on the right. Click Rescan: the .source-row → .skeleton-line transition has no visible row-height jump. About + Updates sections are visible without scrolling on the default 3-section render. Clear first_run_done flag and relaunch: wizard window shows same dark fill, header stays put while body scrolls (test by force-resizing to a smaller height), no light bleed."

files_changed:
  - "src/styles/settings.css (CSS only — single coordinated fix)"


## Symptoms

expected: Both windows feel premium-dark end-to-end. No visible OS scrollbar in the chromeless gap. No light body color bleed. Wizard header sticks while content scrolls. Skeleton rows match real-row dimensions exactly so transitions are seamless. Settings content fits within 520×580 without scrolling.
actual:
  - Settings 520×580: rounded dark card visible; native silver Windows scrollbar to the right of the card; padding around card reveals slightly-lighter (off-white) background.
  - Wizard 480×560: same scrollbar + bg pattern; on scroll the "Welcome to Hallmark" header scrolls with the body (not sticky).
  - Settings: skeleton placeholders shorter than rendered SettingsSourceRow rows — visible jump on rescan resolve.
  - Settings: About + Updates only reachable by scrolling; horizontal space under-used.
errors: None
reproduction: cargo tauri dev → tray right-click → Settings (test 6); clear first_run_done → relaunch (test 14).
started: Discovered during Phase 4 UAT tests 6 + 7 + 14 on 2026-05-09.

## Eliminated

(none yet)

## Evidence

(to be appended as files are inspected)

## Resolution

root_cause:
fix:
verification:
files_changed: []
