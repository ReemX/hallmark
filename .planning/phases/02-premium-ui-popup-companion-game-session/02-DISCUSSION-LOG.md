# Phase 2: Premium UI — Popup, Companion & Game Session - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-08
**Phase:** 2-Premium UI — Popup, Companion & Game Session
**Areas discussed:** Popup signature look & sound; Popup queue timing & 100% rule; Companion window UX; Schema + icon source + rarity tiers

---

## Popup Signature Look & Sound

### Q1: Anchor on screen

| Option | Description | Selected |
|--------|-------------|----------|
| Bottom-left (PS5-style) | Closest to PS5 trophy reference; lower-left corner, slides up + right-in. Risk: bottom-left often holds chat/HUD. | |
| Top-center | Xbox-style. Strong attention but more intrusive over gameplay. | |
| Top-right | Closest to Steam native popup but with premium treatment. Less iconic. | |
| Bottom-right | Out of the way, low-conflict with HUD chat overlays. Less premium feel. | |

**User's choice (free-text):** "actually from what I saw on youtube ps5 either do top right or top left but not at the very top, like a big margin lower, like if you put an horizontal line in the middle of the screen and basically halfed the screen, than half the half, thats where they put it"
**Notes:** User clarified the PS5 anchor is upper-corner ~25% from top, not edge. None of the original 4 options matched.

### Q2 (follow-up): Anchor side

| Option | Description | Selected |
|--------|-------------|----------|
| Top-right (~25% down) | Right-side anchor, vertical ~quarter-from-top, margin from edge. | ✓ |
| Top-left (~25% down) | Left-side anchor, vertical ~quarter-from-top. PS5 itself uses this side more often. | |
| Either is fine — you pick | Claude picks max HUD-safe; would deliberate top-right vs minimap conflict. | |

**User's choice:** Top-right (~25% down)
**Notes:** Final lock — top-right, ~25% from top edge, comfortable margin.

### Q3: Silhouette

| Option | Description | Selected |
|--------|-------------|----------|
| Wide horizontal pill (PS5 stadium) | Full pill, icon left, text right. Closest to PS5 reference. | ✓ |
| Rounded rectangle (Xbox-like) | Soft 12px corners. Clean, modern, less iconic. | |
| Asymmetric — icon notch + extending banner | Icon-disc protrudes; text panel sweeps right. More custom; riskier. | |
| You decide — best fits PS5 inspiration | Claude would pick the wide pill. | |

**User's choice:** Wide horizontal pill (PS5 stadium)

### Q4: Material / color direction

| Option | Description | Selected |
|--------|-------------|----------|
| Dark glass + cool accent (PS5 Pure) | Translucent dark glass, cool white/cyan stroke, white text. PS5 OS-aesthetic. | ✓ |
| Dark glass + gold accent (Trophy) | Same glass but warm gold stroke. Trophy/PSN platinum association. | |
| Solid graphite + chromatic gradient stroke | Opaque dark gray with animated cyan→violet stroke. Cyberpunk premium. | |
| Pure white frosted + dark text | Inverted; better legibility on dark games but lower 'satisfying ding' association. | |

**User's choice:** Dark glass + cool accent (PS5 Pure)

### Q5: Sound character

| Option | Description | Selected |
|--------|-------------|----------|
| Single signature ding | One short signature sound, ~600–1000ms. Simple. Less 'epic' for rare-tier. | |
| Layered: tonal ding + soft riser/whoosh | Two stems mixed. Richer; rare-tier can pitch up or add a third layer. | |
| Layered + rare-tier-only third layer | ding + riser standard; rare adds sparkle/choir. Most flexibility for POPUP-06. | |
| You decide — fits PS5 Pure | Claude would recommend layered base + rare-tier upgrade. | ✓ (deferred to Claude) |

**User's choice (free-text):** "you pick what you think would be most satisfying and rewarding for the player without majorely disturbing is play session"
**Claude's pick (locked into CONTEXT.md):** Layered base — tonal ding + subtle riser/whoosh, ~900ms, peak −8dBFS (sits under voice/dialogue), short attack/fast release. Rare-tier (D-06) adds a third sparkle/choir layer, ~1100ms, peak −5dBFS. Reasoning: layered gives richness for the satisfying-and-rewarding requirement; attenuated peak below voice and short attack/release prevents disturbing play; rare-tier separation fulfills POPUP-06.

### Q6: Animation feel

| Option | Description | Selected |
|--------|-------------|----------|
| Slide-in from right + bounce-settle (PS5 spring) | Slides in from off-screen right, slight overshoot + spring-settle. ~300ms in/out. PS5 reference. | ✓ |
| Fade + scale-up from 0.9 (subtle) | Cross-fade in with scale 0.9→1.0. Calmer; less iconic. | |
| Drop-from-above + settle | Pops in from above the anchor, micro-bounce. Risks looking like a notification toast. | |
| You decide — PS5 spring | Claude would pick the slide-in. | |

**User's choice:** Slide-in from right + bounce-settle (PS5 spring)

---

## Popup Queue Timing & 100% Rule

### Q1: On-screen hold duration

| Option | Description | Selected |
|--------|-------------|----------|
| 3 seconds (PS5 reference) | Closest to PS5 trophy popup. Long enough to read title + first line of desc. | ✓ |
| 5 seconds (generous) | Full description even at 4K. Risk: long during burst. | |
| 2 seconds (snappy) | Quick. Risk: text unreadable for long descriptions. | |
| 4 seconds | Middle ground. | |

**User's choice:** 3 seconds (PS5 reference)

### Q2: Gap between sequential popups

| Option | Description | Selected |
|--------|-------------|----------|
| 200ms (tight, PS5-ish) | Short breath; queue moves quickly. Burst of 5 = ~18s. | ✓ |
| 0ms (back-to-back) | Next slides in as previous slides out. Risk: hectic, animation ambiguity. | |
| 500ms (clear separation) | Each popup is its own moment. Burst of 5 = ~20s. | |
| 1 second | Very clear separation; longest queue. | |

**User's choice:** 200ms (tight, PS5-ish)

### Q3: Burst-cap policy

| Option | Description | Selected |
|--------|-------------|----------|
| No cap — queue all sequentially | Strictest read of POPUP-02. 50 unlocks = ~3min. Locks user out of 'moment'. | |
| Cap visible queue at 5, batch-collapse rest | First N play normally; rest collapses into 'X more achievements unlocked' summary. | |
| Speed up when queue >5 (compress to 1.5s, 0ms gap) | Honors POPUP-02 literally; burst clears in reasonable time; resumes 3s/200ms when queue ≤5. | ✓ |
| You decide | Claude would recommend Option 3. | |

**User's choice:** Speed up when queue >5 (compress on-screen to 1.5s, 0ms gap)

### Q4: 100% celebration trigger frequency

| Option | Description | Selected |
|--------|-------------|----------|
| Once per game ever (persistent) | Stored as SQLite flag. Re-install/replay does not re-trigger. Most 'earned'-feeling. | ✓ |
| Every time a game reaches 100% in a session | Re-triggers if user uninstalls/reinstalls. More forgiving. | |
| Once per game per Hallmark install | Persists until SQLite is wiped. | |
| You decide | Claude would recommend Option 1. | |

**User's choice:** Once per game ever (persistent)

---

## Companion Window UX

### Q1: Window chrome

| Option | Description | Selected |
|--------|-------------|----------|
| Decorated standard window | Native title bar, min/max/close. Familiar, least premium. | |
| Borderless rounded card with custom drag region | Custom-styled, rounded corners, custom close button, draggable header strip. Premium feel; more design work. | ✓ |
| Borderless side-rail (anchored, fixed width) | Pinned slim panel on screen edge. Strong glance utility; less window-like. | |
| You decide | Claude would recommend Option 2. | |

**User's choice:** Borderless rounded card with custom drag region

### Q2: Default size + position

| Option | Description | Selected |
|--------|-------------|----------|
| Medium ~480×720, centered on primary monitor | Tall portrait; ~8 visible. Sensible for first launch. | |
| Compact ~360×600, bottom-right of primary | Smaller, out of way. ~6 visible. Lower 4K utility. | |
| Large ~600×900, centered | Showy first impression; ~12 visible. | |
| You decide | Claude would recommend 480×720 centered, persisted thereafter. | ✓ (deferred to Claude) |

**User's choice:** You decide
**Claude's pick (locked):** 480×720 portrait, centered on primary monitor on first run; SQLite persists size + position thereafter.

### Q3: Always-on-top during gameplay

| Option | Description | Selected |
|--------|-------------|----------|
| Normal (not on-top) — user alt-tabs | Game stays on top; companion sits behind. Cleanest for fullscreen. | ✓ |
| Always-on-top while game runs | Floats over borderless games. Risk: blocks part of game; focus-steal. | |
| Always-on-top + click-through optional toggle | On-top by default; user toggles click-through. More UI complexity. | |
| You decide | Claude would recommend Option 1 — popup is moment surface, companion is alt-tab surface. | |

**User's choice:** Normal (not on-top) — user alt-tabs to see it

### Q4: List interactivity (multi-select)

| Option | Description | Selected |
|--------|-------------|----------|
| Sort: by earned-status / by rarity / by name | Header dropdown switches sort. | |
| Filter: show only earned / locked / all | Quick filter chip at top. | |
| Search by title | Text input filters by display name. | |
| Click row to expand details | Tap to reveal full description + unlock timestamp. | |

**User's choice (free-text):** "you decide whats important for a good v1 release"
**Claude's pick (locked):** Filter chip (All / Earned / Locked) + Sort toggle (Earned-first / A-Z) + Tap-to-expand row. Skipped for v1: text search (premature; few games >100 achievements) and sort-by-rarity (rarity data unreliable per POPUP-07 — would create empty/inconsistent sorts).

---

## Schema + Icon Source + Rarity Tiers

### Q1: Schema/icon resolution source

| Option | Description | Selected |
|--------|-------------|----------|
| Steam public Web API + appcache fallback | Primary anonymous endpoints + appcache fallback. Risk: GetSchemaForGame requires API key. | |
| Local Steam appcache scrape only (offline) | Reads librarycache + stats appcache. Goldberg-only games miss metadata. | |
| Hybrid: appcache first, Web API fallback | Tries local first; falls back to Web API for missing appids. Best coverage. | |
| You decide — needs research flag | Claude flags the WHICH endpoints / key requirements as research-time refinement; locks the policy intent. | ✓ (deferred to Claude) |

**User's choice:** You decide — needs research flag
**Claude's pick (locked):** Hybrid lookup chain: SQLite cache → local Steam appcache → Steam public Web API → cache result. Async on game-start (NOT first popup). Popup uses cached schema and upgrades content in place if resolution completes during 3s hold. Fallback popup (api_name + no icon + no description) when nothing is cached or resolved at fire-time. Research flag carried into plan-phase: Steam Web API key requirements, anonymous endpoints, appcache schema-file empirical format.

### Q2: Rarity tier threshold for richer popup

| Option | Description | Selected |
|--------|-------------|----------|
| Single tier: <10% rare (Steam's own) | Steam UI already labels <10% as rare. Simple binary. | ✓ |
| Two tiers: <10% rare, <2% ultra-rare | Adds third popup tier (ultra) with even richer treatment. More mix work. | |
| Three tiers: <25% / <10% / <2% | Most granular; risks tier inflation. | |
| You decide | Claude would recommend Option 1 binary. | |

**User's choice:** Single tier: <10% = rare (Steam's own threshold)

---

## Claude's Discretion

User explicitly deferred to Claude on:
- Sound character (Q5 of Popup look & sound) — picked layered base + rare-tier sparkle add, with peak attenuation policy.
- Companion default size/position (Q2 of Companion UX) — picked 480×720 centered, persisted thereafter.
- Companion list interactivity (Q4 of Companion UX) — picked filter + sort + tap-to-expand; skipped search and rarity-sort.
- Schema/icon source (Q1 of Schema area) — picked hybrid appcache→Web API chain with cache; flagged endpoint research.

In addition to the above selections, Claude has flexibility (per CONTEXT.md `## Implementation Decisions` → `### Claude's Discretion`) on icon framing within the pill, typography stack, bundled SFX format, icon-disc shape, 100% celebration variant specifics, and `schema_cache`/`icon_cache` table design.

## Deferred Ideas

- Ultra-rare third tier (POPUP-V2 candidate)
- Sort-by-rarity in companion (gated on rarity data coverage)
- Companion text search (defer until games >100 achievements)
- Companion click-through on-top toggle (matches future QOL-V2-02 streamer/privacy mode)
- Custom theme system / sound replacement (out of scope per PROJECT.md, not re-decided here)
