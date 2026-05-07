# Empirical Goldberg State File Schema (Assumption A4 Resolution)

**Date:** 2026-05-08
**Resolves:** RESEARCH.md Assumption A4 (LOW confidence — gbe_fork field-name divergence from legacy Goldberg), Open Question #1
**Status:** SCHEMA CONFIRMED — gbe_fork output on this machine matches the field names documented for legacy Goldberg in three independent secondary sources.

## Method

Ran two PowerShell scans against the two known Goldberg/gbe_fork save roots on
the developer machine to surface any locally-installed real saves, then
inspected each surfaced file's raw JSON to verify field names, value types,
and top-level shape.

Commands executed (recorded verbatim so this is reproducible):

```powershell
Get-ChildItem -Path "$env:APPDATA\Goldberg SteamEmu Saves" -Recurse -Filter achievements.json -ErrorAction SilentlyContinue | Select-Object -First 3 FullName
Get-ChildItem -Path "$env:APPDATA\GSE Saves"               -Recurse -Filter achievements.json -ErrorAction SilentlyContinue | Select-Object -First 3 FullName
```

Followed by `Get-Content <path> -Raw` on each surfaced file to dump raw JSON.

## Real Saves Inspected

Three real saves were found under `%APPDATA%\GSE Saves\` (the gbe_fork default
root). Zero saves were found under `%APPDATA%\Goldberg SteamEmu Saves\` (the
legacy default root) on this machine — the directory exists but contains no
`achievements.json` files. This matches the research finding that gbe_fork has
become the dominant fork for 2024+ scene releases (PITFALLS.md #3).

### Save 1: `C:\Users\reema\AppData\Roaming\GSE Saves\1455840\achievements.json`

```json
{
  "analyst_00": {
    "earned": false,
    "earned_time": 0
  },
  "analyst_01": {
    "earned": false,
    "earned_time": 0
  },
  "biomes_00": {
    "earned": false,
    "earned_time": 0
  },
  ... (all entries follow identical shape; truncated for brevity)
}
```

### Save 2: `C:\Users\reema\AppData\Roaming\GSE Saves\1948280\achievements.json`

```json
{
  "befriend_a_pirate": {
    "earned": false,
    "earned_time": 0
  },
  "build_house": {
    "earned": true,
    "earned_time": 1759509789
  },
  "create_offspring": {
    "earned": true,
    "earned_time": 1759510615
  },
  ... (truncated)
}
```

### Save 3: `C:\Users\reema\AppData\Roaming\GSE Saves\2592160\achievements.json`

```json
{
  "ACH_CAR": {
    "earned": false,
    "earned_time": 0
  },
  "ACH_CONVOY": {
    "earned": true,
    "earned_time": 1764373966
  },
  "ACH_FIRST_SHOT": {
    "earned": true,
    "earned_time": 1764340245
  },
  "ACH_HACK_FAST": {
    "earned": true,
    "earned_time": 1764328522
  },
  ... (truncated)
}
```

All three files use the identical schema: `{ "<api_name>": { "earned": <bool>, "earned_time": <u64-unix-seconds> } }`.

## Field Names Confirmed

| Field | Type | Found in legacy Goldberg | Found in gbe_fork | Notes |
|-------|------|--------------------------|-------------------|-------|
| `earned` | `bool` (`true`/`false`) | YES (per RESEARCH.md secondary sources: xan105/Achievement-Watcher wiki, achievement-watchdog README, Goldberg `Readme_release.txt`) | YES (3/3 saves on this machine) | Primary unlock signal — the `false → true` transition is what fires events. Same name, same type, in both forks. |
| `earned_time` | `u64` (Unix epoch seconds) | YES (per same three secondary sources) | YES (3/3 saves on this machine) | `0` indicates "earned but timestamp unknown" or "not yet earned". MUST NOT be used as the unlock signal — see PITFALLS.md #15. Save 2 (`1948280`) shows clear evidence of valid timestamps (`1759509789` ≈ 2025-10-03) coexisting with `0` for unearned entries. |

No additional or unexpected fields were observed in any of the three real saves.

## Top-level Shape

OBJECT (map of `api_name` → entry). NOT an array. Each entry is itself an
object with exactly the two fields above. Confirmed across all three real
saves and consistent with RESEARCH.md "Goldberg State File Schema".

## Decision for Plan 04

**Schema confirmed — Plan 04 locks parser to `{ "<api_name>": { "earned": bool, "earned_time": u64 } }`.**

Specifically:
- `serde` struct: `#[derive(Deserialize)] struct GoldbergEntry { earned: bool, earned_time: u64 }`.
- Top-level: `HashMap<String, GoldbergEntry>` deserialized via `serde_json::from_str`.
- Unlock signal: `earned` boolean transition `false → true` against the in-memory baseline. `earned_time` is captured for telemetry only and MUST NOT gate event emission (PITFALLS.md #15).

Despite the empirical confirmation, Plan 04 SHOULD still apply two defensive
measures because the gbe_fork project is actively developed and a future
release could add fields or change shape:

1. `#[serde(default)]` on any non-essential field added later (and on
   `earned_time` itself for pre-existing files that pre-date this convention,
   though no such files were found in this scan).
2. A `serde_json::Value` escape hatch on parse failure: if strict
   deserialization fails for a single file, log a structured warning and skip
   that file, rather than panicking the watcher.

These keep the parser tolerant to future divergence without weakening the
strict-typed happy path.

## Conservative Fallback

Not needed in this case — three real gbe_fork saves were available on this
machine, so A4 is resolved by direct observation rather than secondary
sources. The fallback paragraph from the original plan applies only when no
real save is reachable.
