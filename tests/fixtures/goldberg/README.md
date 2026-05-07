# Goldberg State File Fixtures

This directory contains canonical Goldberg / gbe_fork state file fixtures used by
downstream parser, watcher, and dedup tests. The file at `480/achievements.json`
is the **STATE** file that emulators write at runtime to record achievement
unlock progress (object map keyed by achievement API name) — it is **not** the
SCHEMA file (an array of `{name, displayName, description, icon, hidden}`
objects that lives next to `steam_api.dll` under `<game-dir>\steam_settings\`).
Confusing the two is Pitfall #4 in `.planning/research/PITFALLS.md`.

The directory name `480` is the Steam appid for **Spacewar**, the official
Steamworks SDK demo title. Spacewar is a convention-safe non-real-game appid
that Valve publishes for SDK testing, so using it as a fixture appid avoids any
implication that this project targets a specific commercial title. Downstream
tests treat this fixture as an immutable reference — modifying it without
updating those tests will silently break them.

The fixture intentionally covers four cases so the parser and dedup logic can
be exercised end-to-end:

| Key | earned | earned_time | What it tests |
|-----|--------|-------------|---------------|
| `ACH_WIN_ONE_GAME` | `true` | `1700000001` | Earned with a real timestamp (the happy path). |
| `ACH_WIN_100_GAMES` | `false` | `0` | Unearned baseline; the dedup baseline must include this. |
| `ACH_TRAVEL_FAR_ACCUM` | `false` | `0` | Second unearned entry, used to verify multi-entry diffs. |
| `ACH_UNKNOWN_TIMESTAMP` | `true` | `0` | Earned but timestamp is `0` — covers PITFALLS.md #15. The `earned: bool` transition is the only valid unlock signal; `earned_time` MUST NOT be used to detect unlocks. |

Empirical confirmation that this schema matches real gbe_fork output (resolving
RESEARCH.md Assumption A4) is recorded at
`.planning/phases/01-detection-pipeline-foundation/empirical-goldberg-schema-NOTES.md`.
