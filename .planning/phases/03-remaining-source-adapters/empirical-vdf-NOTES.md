# Empirical Steam-Legit Binary VDF Path + Schema (REQ DETECT-02 Resolution)

**Date:** 2026-05-09
**Resolves:** REQUIREMENTS.md DETECT-02 path discrepancy (cited `userdata/<steamid>/<appid>/remote/` — incorrect; the actual achievement state path is `appcache/stats/UserGameStats_<userid>_<appid>.bin`)
**Status:** PATH CONFIRMED — empirical inspection of 166 real `UserGameStats_*.bin` files at `C:\Program Files (x86)\Steam\appcache\stats` on the dev machine + canonical OSS sources (xan105/Achievement-Watcher `app/parser/steam.js` + hydralauncher/hydra) all agree.

## Method

Re-validation PowerShell scans run on dev machine 2026-05-09:

1. State files at `appcache/stats` (`Get-ChildItem 'C:\Program Files (x86)\Steam\appcache\stats' -Filter 'UserGameStats_*.bin' | Select-Object -First 3`):
```
Name                                Length LastWriteTime
----                                ------ -------------
UserGameStats_132274694_1013910.bin    101 4/9/2026 7:18:56 PM
UserGameStats_132274694_105600.bin     191 10/30/2025 9:04:12 AM
UserGameStats_132274694_1061090.bin     78 11/20/2025 6:58:11 PM
```

2. Schema files at `appcache/stats` (`Get-ChildItem 'C:\Program Files (x86)\Steam\appcache\stats' -Filter 'UserGameStatsSchema_*.bin' | Select-Object -First 3`):
```
Name                            Length LastWriteTime
----                            ------ -------------
UserGameStatsSchema_1013910.bin  28209 4/9/2026 7:18:56 PM
UserGameStatsSchema_105600.bin   41206 10/30/2025 9:04:12 AM
UserGameStatsSchema_1061090.bin  18314 11/20/2025 6:58:11 PM
```

3. Hex dump of first 32 bytes of one state file `UserGameStats_132274694_1013910.bin` (verifies binary VDF, not JSON/text):
```
00 63 61 63 68 65 00 02 63 72 63 00 CD 2C 30 DB 02 50 65 6E 64 69 6E 67 43 68 61 6E 67 65 73 00
```

The leading bytes match the binary VDF header `\0cache\0` (0x00 0x63 0x61 0x63 0x68 0x65 0x00) followed by Int32 type-tag (0x02) entries — at offset 7 we see `02` (Int32 tag) + `crc\0` + 4-byte CRC value (`CD 2C 30 DB`), then `02` (Int32 tag) + `PendingChanges\0`. This is the format adapter 03-01 will parse with `vdf_binary::parse_binary_vdf`.

## Decision for Plan 03-01

Plan 03-01 (SteamLegitAdapter) MUST:
- Watch path: `<SteamPath>\appcache\stats` (recursively, but the dir is flat — no subdirs).
- File pattern: `UserGameStats_<userid>_<appid>.bin` (state). Filename guard at start of `on_file_changed`.
- Schema lookup: `UserGameStatsSchema_<appid>.bin` in same dir, mtime-cached.
- Filename pattern parse via regex or split: `^UserGameStats_(\d+)_(\d+)\.bin$` -> capture groups (user_id, app_id).
- DO NOT watch `userdata/<steamid>/<appid>/remote/` — that is Steam Cloud save data, not achievements.

## Type Tags Confirmed Present

Per the hex dump above and RESEARCH.md inspection, the binary VDF type tags observed in achievement files are:
- 0x00 Object — used for `cache` root + per-stat-slot subobjects + `AchievementTimes` map
- 0x01 String — used in schema files for achievement API names + localized labels
- 0x02 Int32 — used for `crc`, `PendingChanges`, per-stat-slot `data`, and AchievementTimes timestamp values
- 0x08 ObjectEnd — closes each Object scope

Type tags 0x03 (Float), 0x07 (UInt64) are documented but not observed in the inspected sample. Plan 03-01's `vdf_binary.rs` MUST handle all 8 documented tags; unknown tags log warn + skip per Pitfall #2.

## Conservative Fallback

Not needed — empirical evidence is direct.

## REQUIREMENTS.md Fix Applied

DETECT-02 was rewritten in this same plan (see git log) to remove the misleading `userdata/<steamid>/<appid>/remote/` reference and cite `appcache/stats/UserGameStats_<userid>_<appid>.bin` instead. The corrected line is the source of truth going forward.
