# Hallmark

## What This Is

Hallmark is a Windows app that brings PSN/Xbox-grade achievement satisfaction to PC gaming. When a Steam achievement unlocks — whether from a legitimate copy or a Goldberg/CreamAPI-emulated copy — Hallmark fires a premium signature-style popup with a godly sound effect, and shows a session-focused companion view of the current game's achievements. For PC gamers who want the moment-to-moment payoff that current PC achievement systems rarely deliver.

## Core Value

Make PC achievement unlocks feel as satisfying as a PS5 trophy ding — every time, in every supported game.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- Current scope. Building toward these. -->

- [ ] Real-time detection of legitimate Steam achievement unlocks via local file watcher
- [ ] Real-time detection of Goldberg / CreamAPI / SmartSteamEmu unlocks via the same file watcher (transparent — no installer or setup help)
- [ ] Premium signature-style in-game popup overlay (PS5-inspired, designer-locked look)
- [ ] Godly signature sound effect on unlock
- [ ] Session-focused companion view: auto-shows when a game launches, hides when it closes
- [ ] Companion view displays current game's achievement list (earned, available, rarity if known)
- [ ] Game-launch detection: hybrid (Steam state when available, OS process scanner fallback)
- [ ] External borderless always-on-top overlay window for popup rendering (no DLL injection in v1)
- [ ] Achievement icon, title, and description rendered in popup
- [ ] Local-only operation — no cloud, no accounts, no sync, no telemetry
- [ ] Free, open-source distribution on GitHub

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- DLL injection overlay — deferred to v2; external borderless window covers the common case (modern games run borderless/windowed) without anti-cheat risk or per-renderer hook complexity
- Lifetime stats, leaderboards, social/community features — user's actual usage is moment-to-moment; lifetime profile is not the driver
- Cloud sync, accounts, profiles — local-only by design
- Theme presets and sound customization — signature style locked deliberately for brand identity; "premium feel" requires designer control over the unified look
- Goldberg/CreamAPI setup assistance — passive detection only; app does not install or configure Steam emulators
- Stores beyond Steam (Epic, GOG Galaxy, Xbox/MS Store, Ubisoft Connect, EA App, Battle.net) — community can contribute later via plugin/source-adapter pattern
- macOS, Linux, Steam Deck — Windows v1; reduce scope
- Steam Web API integration — file watcher gives real-time, offline, emulator-friendly detection; Web API polling has 1–5 min lag that breaks the popup feel
- Screenshot-on-unlock, deep stats, session history archive
- Steam-only DRM-protected achievement bypass tooling — not a goal

## Context

- Existing Steam, Epic, and GOG achievement notifications feel disposable: muted sounds, small popups, sometimes silent. Console achievement systems (PS5 trophies, Xbox achievements) deliver substantially more satisfaction per unlock — that gap is the opportunity.
- Existing PC tooling splits into two categories. Aggregator dashboards (Exophase, MetaGamerScore, TrueSteamAchievements, PlayTracker, Trophies Hunter, AStats) are read-only profile sites. Notification parsers (Achievement Watcher by xan105, Playnite SuccessStory plugin, PSerban93/Achievements) catch unlock events but ship hobby-grade UX. None deliver a premium console-grade in-game popup as the hero feature.
- RetroAchievements proves a community-grade achievement layer with overlay can succeed at scale (~10k titles), but is locked to emulated retro games.
- Goldberg / CreamAPI / SmartSteamEmu are widely-deployed Steam emulator DLLs. They write achievement state to predictable local file paths (e.g. `Documents/Goldberg SteamEmu Saves/<appid>/achievements.json`). A single file watcher can therefore detect unlocks from both legitimate `Steam/userdata/<id>/<appid>/...` paths and emulator paths transparently.
- The killer differentiator is moment-to-moment popup-and-sound feel, not feature breadth. Signature-style design with no theme knobs is a deliberate brand-identity choice.
- Public release goal: free, open-source, hobby-pace. Community-extensible architecture (additional store source-adapters) is desirable but not required for v1.

## Constraints

- **Platform**: Windows-only for v1 — where the games are, where the file paths and process model are stable.
- **Overlay tech**: External borderless always-on-top window, no DLL injection — trades exclusive-fullscreen edge cases for fast ship and zero anti-cheat risk.
- **Detection**: Local file watcher only (no Steam Web API in v1) — required for real-time popup latency, offline support, and emulator coverage in one mechanism.
- **Distribution**: Free, open-source on GitHub — community-extensible store coverage is the long-term path.
- **Goldberg / emulator stance**: Passive detection only — app reads emulator output paths if they exist; does not install, configure, or recommend emulator setup.
- **Customization**: Signature style locked — no user-editable themes, sounds, positions, or animations in v1.
- **Pace**: Hobby project — polish over speed; no fixed deadline.

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| External overlay window over DLL injection | Ships fast, zero anti-cheat risk, modern games mostly run borderless/windowed | — Pending |
| File watcher only (no Steam Web API in v1) | Real-time, covers legit + emulated, no online dependency; API polling lag breaks popup feel | — Pending |
| Signature style only (no theme system) | Brand identity over flexibility; premium feel requires designer control | — Pending |
| Steam (legit) + Goldberg/CreamAPI/SmartSteamEmu only at v1 | Largest catalog + biggest experience gap; one watcher mechanism covers both | — Pending |
| Free + open-source | Trust, virality, community-contributed store source-adapters later | — Pending |
| Local-only, no cloud | User's actual usage is moment-to-moment; lifetime profile is not the driver | — Pending |
| Windows-only v1 | Where the games are; minimize surface area | — Pending |
| Session-focused companion (not lifetime dashboard) | Matches the moment-to-moment usage pattern | — Pending |
| Hybrid game-launch detection (Steam state + process scanner) | Steam state gives accurate appID when available; process scanner covers Goldberg/non-Steam launches | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-07 after initialization*
