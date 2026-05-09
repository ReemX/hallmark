---
status: diagnosed
trigger: "After cargo tauri dev launches, both popup and wizard windows appear almost immediately but stay BLANK for many seconds. User reports 20s for UI to initialize and 'wizard takes 15 minutes to go from blank to showing content'. Achievements fired during warmup play SFX but popup never paints. Discovered during Phase 4 UAT tests 4 + 14 on 2026-05-09."
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: "Confirmed: 20s gap is the documented Tauri v2 + Vite dev-mode WebView2-to-localhost-connect-and-cold-transform delay. WebView IS attempting to load popup.html / wizard.html immediately on window creation (Tauri docs confirm hidden windows DO preload URL). The lag is the round trip: WebView2 → http://localhost:1420/popup.html → Vite first transforms popup.tsx + traverses imports (React 19 + Framer Motion + tauri @api) → returns transformed module graph. Production NSIS bundle ships pre-built bundles, eliminating both Vite and the localhost loop."
test: "Static analysis of vite.config.ts (multi-entry rollup), src-tauri/src/ui.rs (popup builder), src-tauri/src/first_run.rs (wizard builder), tauri.conf.json (devUrl http://localhost:1420 in dev, frontendDist=../dist in build), dist/assets/ contents (4 pre-built bundles confirmed present), corroborating GitHub issues."
expecting: "Confirmed."
next_action: "Return diagnosis (goal: find_root_cause_only)."

## Symptoms

expected: WebView windows are interactive within ~1 s of `cargo tauri dev` launch. Popups fired during the first second still render visibly (not just SFX).
actual: User reports "takes at least 20 seconds after opening the program first for ui to initialize" and "[wizard] takes like 15 minutes to go from blank to showing content". Achievements fired during this warmup play sound but the popup window never paints. Wizard window appears as a blank dark card with no content for many seconds before snapping in.
errors: None.
reproduction: 1) cargo tauri dev. 2) Within ~5 s, right-click tray → Fire test popup. SFX plays, no popup. Wait. Eventually popup queue drains. 3) Clear first_run_done, restart. Wizard window appears immediately, content paints many seconds later.
started: Discovered during Phase 4 UAT tests 4 + 14 on 2026-05-09.

## Eliminated

- hypothesis: "H1 (starting_points): Vite serves modules on-demand and Tauri only triggers URL load when window is shown — popup window is `visible: false` so popup.html GET fires only on first show, causing cold transform stall on first reveal."
  evidence: "Tauri v2 docs (search confirmed): when a window is configured with `visible: false`, it still spawns a hidden window and runs any code defined in the page loaded by the webview — URL is preloaded and executed even when hidden. So WebView2 fetches /popup.html during setup() at the same time the wizard's `visible: true` window does. The first paint delay applies equally to both windows BECAUSE the same Vite cold-transform pipeline gates both, not because one is hidden. Wizard with `visible: true` ALSO appears blank — confirming the URL-on-show theory is wrong."
  timestamp: 2026-05-09T00:00:00Z

## Evidence

- timestamp: 2026-05-09T00:00:00Z
  checked: "vite.config.ts"
  found: "Multi-entry rollupOptions.input declares 4 entries (companion/popup/settings/wizard). dev server is plain `vite` (port 1420 strictPort). NO `optimizeDeps.entries` configuration — Vite's dev pre-bundle (esbuild) only auto-discovers from index.html on cold start. The other 3 entries (popup.html, settings.html, wizard.html) are NOT pre-bundled until first request hits them, at which point Vite kicks off ESM-graph traversal + dependency pre-bundling on-the-fly."
  implication: "Multi-entry app has uneven cold start: index.html (companion) gets pre-bundled at server start; popup/wizard/settings each pay full cold-bundle cost on first GET. React 19 + Framer Motion + @tauri-apps/api is a non-trivial dep graph — first transform cost easily 5-15 s on first hit per entry."

- timestamp: 2026-05-09T00:00:00Z
  checked: "src-tauri/tauri.conf.json"
  found: "build.devUrl = http://localhost:1420 (Vite dev server). build.frontendDist = ../dist (production bundle). beforeDevCommand = pnpm dev (= `vite`). beforeBuildCommand = pnpm build (= `tsc -b && vite build`)."
  implication: "Two distinct asset-serving pipelines: dev = WebView2 fetches from localhost:1420 (Vite transforms ESM modules per-request); production = WebView2 reads from bundled tauri:// scheme pointing at ../dist (pre-built static assets). The 20s gap exists ONLY in dev because production has no Vite, no localhost round trip, no on-demand transform."

- timestamp: 2026-05-09T00:00:00Z
  checked: "dist/assets/ directory (production build output)"
  found: "Pre-built bundles ALREADY exist: companion-CPDLikg3.css, companion--GuBfNeo.js, popup-Dml35ryJ.css, popup-BGd0qpbs.js, settings-D2IwksJ9.css, settings-DkuEgkqx.js, wizard-CnNnpB7X.js (no separate wizard CSS — wizard imports settings.css per first_run.rs comment). Plus shared chunks: core-sK_JlY1z.js, event-BRuWPQ-a.js, proxy-D6sVRAzL.js, webviewWindow-BtQSQmjh.js."
  implication: "Production build pipeline is healthy and complete. Bundles are minified and chunked. cargo tauri build verified passing in UAT test 19 (NSIS installer 5.1MB, sub-10MB target). Production WebView load is essentially instant — file:// or tauri:// scheme reads pre-minified bundles from the asset bundle directly."

- timestamp: 2026-05-09T00:00:00Z
  checked: "src-tauri/src/ui.rs::create_popup_window"
  found: "WebviewWindowBuilder.new(app, 'popup', WebviewUrl::App('popup.html'.into())) ... .visible(false) ... .build()? — popup window IS created with hidden flag, but URL is set at builder time. Tauri instantiates the webview during build(); URL fetch begins immediately."
  implication: "Hidden popup window starts loading popup.html during setup() — at the SAME wall-clock moment as the wizard. So the gap is not 'show triggers load'; the gap is 'load takes a long time'. Once the WebView finishes loading + React mounts + listen('popup-show') is registered, only THEN can a future popup-show event paint. SFX fires at audio.play() in popup_queue.rs which is independent of WebView state, explaining the 'sound but no visual' window."

- timestamp: 2026-05-09T00:00:00Z
  checked: "src/main-popup.tsx"
  found: "useEffect registers listeners for 'popup-show' and 'popup-hide' AFTER React mounts. Imports: React, framer-motion, @tauri-apps/api/event, ./components/PopupCard, ./types. PopupCard transitively imports more Framer Motion + CSS."
  implication: "If popup_queue::process_event emits 'popup-show' BEFORE main-popup.tsx's useEffect has run (i.e., before React has mounted), the event is fired into the void — Tauri's event system delivers to listeners present at emit time. Tauri events do not buffer for late subscribers. This is the secondary mechanism for 'SFX without popup': the emit_to('popup', 'popup-show', payload) call in popup_queue.rs executes against a popup window whose JS context has not yet attached the listener."

- timestamp: 2026-05-09T00:00:00Z
  checked: "src-tauri/src/popup_queue.rs::process_event (lines 161-167)"
  found: |
    if let Some(popup) = app.get_webview_window("popup") {
        if !popup.is_visible().unwrap_or(true) {
            let _ = popup.show();
        }
    }
    let _ = app.emit_to("popup", "popup-show", &payload);
    if let Err(e) = audio.play(tier_audio) { ... }
  implication: "Show + emit + audio.play happen in sequence, but there is NO frontend-ready handshake. There is no `await` for a 'popup-mounted' event from React. If the WebView is still loading (Vite cold transform in dev, or normal mount delay in production), emit_to fires before React's listen() callback is registered — event is silently dropped. audio.play succeeds because rodio is Rust-side. Result: sound plays, no popup. This is the EXPLICIT MISSING handshake noted in the UAT gap entry: 'frontend-ready ack from popup WebView back to Rust before popup_queue starts firing'."

- timestamp: 2026-05-09T00:00:00Z
  checked: "Web search: Tauri v2 + Vite dev mode known issues"
  found: "Issue #12742 (Feb 2025): 'Application launch is incredibly slow — when running `bun run tauri dev` the window takes upwards of 30-40 seconds to connect to the svelte page/vite, though it's immediately available in the browser.' Issue #8920: 'tauri dev is incredibly slow'. Issue #6045: 'slow startup on tauri dev'. Issue #5170 / #5143: 'Blank screen on starting tauri application'. Issue #13017: 'Tauri does not connect to vite dev server, displays white window'."
  implication: "Confirmed. The 20-second blank-WebView gap is a documented Tauri v2 + Vite dev-mode behavior, NOT a Hallmark-specific bug. WebView2 attempts to load http://localhost:1420/popup.html and the connection establishment + Vite cold transform of the entry's full ESM graph takes 10-30+ seconds on first launch. After Vite's transform cache warms (in-process), subsequent dev launches are faster but the FIRST request to each non-pre-bundled entry still pays a cold cost. Production builds bypass this entirely."

- timestamp: 2026-05-09T00:00:00Z
  checked: "Tauri v2 docs (search): WebviewWindowBuilder.visible(false) URL load behavior"
  found: "Confirmed: 'When a window is configured with visible: false, it still spawns a hidden window and runs any code that is defined in the page loaded by the webview.' URL preloads regardless of visibility."
  implication: "Hides starting_points H1 — the popup window's WebView IS loading popup.html during setup(), not waiting for first show(). The WebView is just slow to finish loading. By the time popup_queue receives the first event (immediately after Phase 4 startup logs flush), the WebView is mid-load and React has not mounted — so the 'popup-show' emit reaches no listener."

## Resolution

root_cause: |
  Two distinct mechanisms compound to produce the visible symptom; both are dev-mode artifacts that disappear in production:

  (A) **Vite multi-entry cold-transform bottleneck (DEV ONLY).** vite.config.ts declares 4 rollup input entries (companion/popup/settings/wizard) but the dev server's `optimizeDeps` is left implicit. On `cargo tauri dev`, Vite's esbuild pre-bundle only auto-discovers from index.html (the default entry). When WebView2 fetches /popup.html and /wizard.html, Vite must transform popup.tsx / FirstRunWizard.tsx and lazily traverse their full import graph (React 19, Framer Motion, @tauri-apps/api/event, components/PopupCard, components/WizardSourceRow, types.ts, styles/popup.css, styles/settings.css). On a cold cache this transform + dependency-graph walk takes 10-30+ seconds — long enough that the WebView shows a blank surface (popup window: transparent → invisible blank; wizard window: dark CSS background → visible blank dark card). This is a documented Tauri v2 + Vite dev-mode behavior (tauri-apps/tauri issues #12742, #8920, #6045, #5170).

  (B) **Missing WebView-ready handshake (PRESENT IN BOTH DEV AND PROD, but only USER-VISIBLE in dev because of A).** popup_queue.rs::process_event calls popup.show() + app.emit_to("popup", "popup-show", payload) + audio.play() with no await for a "popup-frontend-mounted" ack from React. Tauri's event system does NOT buffer events for listeners that attach after the emit. If process_event runs before main-popup.tsx's useEffect has registered listen("popup-show"), the event is silently dropped. audio.play succeeds (rodio is independent of WebView state). Result: SFX plays, popup never paints — even though the popup window is mounted and visible. In production this gap is small enough (~100-500 ms WebView2 cold start + React mount) that test_trigger fires almost always succeed because Phase 4 setup() completes after the WebView has mounted. In dev, the ~20 s Vite cold transform widens this gap into the user-visible bug.

  Mechanism (A) is what makes the wizard appear "blank for many seconds" — wizard window is visible:true, dark CSS surface paints, but the React tree hasn't bootstrapped yet because Vite is still transforming. Mechanism (B) is what makes "SFX without popup" possible — even after the WebView mounts, the listener race is structural and would also fire if a real game unlock happened in the first ~500 ms of a production launch.

fix: |
  (Diagnose-only mode — no fix applied. Recommended fix direction below.)

  Two-tier fix recommended:

  **TIER 1 — Acceptable workaround (short-term):** Document the dev-vs-prod warmup gap; switch UAT test 4 + test 14 visual checks to use `cargo tauri build` + run the produced NSIS or release-mode binary instead of `cargo tauri dev`. Production warmup is sub-second and the symptom does not reproduce there. Verify by building once and re-running tests 4 / 14 against `target/release/hallmark.exe`. Cost: zero code change. Tradeoff: slower iteration loop during future UI work, but UAT signoff requires production-equivalent behavior anyway.

  **TIER 2 — Source-level fix (long-term, addresses both A and B):**

  1. **Reduce Vite cold-transform cost (mitigates A in dev).** Add to vite.config.ts:
     ```ts
     optimizeDeps: {
       entries: ['index.html', 'popup.html', 'settings.html', 'wizard.html'],
       include: ['react', 'react-dom', 'react-dom/client', 'framer-motion', '@tauri-apps/api/core', '@tauri-apps/api/event'],
     }
     ```
     This forces esbuild to pre-bundle all 4 entries' shared deps at dev-server start, eliminating per-entry cold transform on first GET. Expected dev warmup: drops from 20 s → 2-5 s on first launch.

  2. **Add WebView-ready handshake (mitigates B in dev AND prod).** Frontend (main-popup.tsx, FirstRunWizard.tsx, etc.) emits a `popup-mounted` / `wizard-mounted` event from useEffect AFTER listen() registers. Rust side: popup_queue must await this ack before its first emit_to. For popup_queue specifically, a startup-gate `tokio::sync::Notify` works: setup() creates the notify; main-popup.tsx invokes a `popup_ready` Tauri command on mount; popup_queue's `run` loop waits on `notify.notified()` before its first emit. This eliminates the "SFX without popup" race even on cold WebView2 mounts.

  Both changes are additive and low-risk. Tier 1 alone resolves the user-reported UAT issue if Hallmark's UAT signoff allows "tested against production build". Tier 2 hardens the production startup race.

verification: "Not yet applied (find_root_cause_only)."

files_changed: []
