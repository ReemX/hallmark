---
phase: 03
status: fixed
files_reviewed: 10
critical: 2
warning: 8
info: 6
depth: standard
reviewed: 2026-05-09T10:30:00Z
fixed: 2026-05-09T11:30:00Z
fixes_applied: 10
---

# Phase 03: Remaining Source Adapters — Code Review Report

**Reviewed:** 2026-05-09T10:30:00Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Phase 03 ships four hand-rolled binary/text parsers and three new adapters wired into the production pipeline. The adapters mostly follow Goldberg's pattern but several BL-02 commit-on-success ordering invariants are violated: the baseline is updated even when `tx.send(evt)` fails (events lost forever), and the SHA-256 short-circuit has a TOCTOU race that allows duplicate events through on concurrent invocations. The integration tests are also weaker than they appear — SC1 measures the wrong latency, SC3-supplement silently tolerates dedup leaks, and EnvGuard is racey across parallel tests.

## Critical Issues

### CR-01: `tx.send` failure still commits baseline (BL-02 violation) — events lost permanently

**Files:**
- `src-tauri/src/sources/cream_api.rs:248-252`
- `src-tauri/src/sources/steam_legit.rs:446-450`
- `src-tauri/src/sources/sse.rs:354-358`

**Issue:** All three adapters log an error when the receiver is dropped but unconditionally call `baseline.insert(key, earned_now)` on the next line. Because the baseline now records `earned = true`, the next `false→true` transition for the same key will not fire (no diff). If the channel was transiently full or the consumer task was being recycled, the unlock event is permanently lost.

The Phase 03-01/02/03 SUMMARY documents call out "BL-02 commit-on-success ordering: emit → THEN update baseline" as preserved verbatim, but the implementation's "emit" branch contains an `if tx.send(...).await.is_err()` that does NOT skip the update.

```rust
// cream_api.rs (and steam_legit.rs, sse.rs):
if !was && earned_now {
    let evt = ...;
    if tx.send(evt).await.is_err() {
        tracing::error!("RawUnlockEvent receiver dropped");
        // ← falls through to insert below
    }
}
baseline.insert(key, earned_now);   // ← runs whether send succeeded, failed, or was skipped
```

**Fix:** Either propagate the error (`return Err(...)`) or skip the baseline update on send failure so a future watcher event re-tries the diff:

```rust
if !was && earned_now {
    let evt = ...;
    if let Err(e) = tx.send(evt).await {
        tracing::error!(error = %e, "RawUnlockEvent receiver dropped; not committing baseline so retry can fire");
        continue; // skip baseline.insert for this entry
    }
}
baseline.insert(key, earned_now);
```

---

### CR-02: SHA-256 short-circuit has TOCTOU race; concurrent calls duplicate events

**Files:**
- `src-tauri/src/sources/cream_api.rs:226-254`
- `src-tauri/src/sources/steam_legit.rs:412-452`
- `src-tauri/src/sources/sse.rs:325-360`

**Issue:** The flow is:
1. Acquire `last_hash` read lock → check membership → drop read lock.
2. (gap with no lock)
3. Acquire `baseline` write lock → emit + update.
4. Acquire `last_hash` write lock → insert hash.

Two concurrent `on_file_changed` invocations on the same path can both observe "hash not present" in step 1 and both proceed to emit duplicates in step 3. The Phase 03 documentation claims "the BL-02 invariant is preserved verbatim," but the per-path SHA-256 short-circuit is no longer atomic with the emit. CrossSourceDedup downstream catches the duplicate based on `(app_id, ach_api_name)`, but only within a 10s TTL — beyond TTL or for distinct keys this leak through. The Phase 03-04 dedup test specifically relies on this short-circuit working as documented.

In practice, `notify-debouncer-full` typically delivers one event per debounce window per path, but the watcher dispatch in Phase 1 was not audited as serializing per-path. The contract should be defensive.

**Fix:** Hold a single lock (or update the hash before emitting). Simplest fix: write the hash entry first as a "claim" before parsing, and CAS-style check:

```rust
let hash: [u8; 32] = Sha256::digest(text.as_bytes()).into();
{
    let mut h = self.last_hash.write().await;  // write lock
    if h.get(&path) == Some(&hash) { return Ok(()); }
    h.insert(path.clone(), hash);  // claim before parse/emit
}
// proceed with parse + emit while holding no lock on last_hash
```

If parse/emit fails, the hash claim stays — next event sees identical bytes and skips, which is correct. If a real change comes after the claim, its different hash will update the entry.

## Warnings

### WR-01: `tx.send().await` held while baseline write lock is taken — backpressure deadlock risk

**Files:**
- `src-tauri/src/sources/cream_api.rs:237-253`
- `src-tauri/src/sources/steam_legit.rs:431-451`
- `src-tauri/src/sources/sse.rs:344-359`

**Issue:** All three adapters call `tx.send(evt).await` inside the loop body where `baseline` write lock is held (cream_api, steam_legit) or re-acquired per record (sse). If the downstream channel fills (e.g., debounced burst) and `send().await` blocks, the baseline lock is held for the duration. Other tasks (e.g., a parallel `on_file_changed` for a different path that needs the baseline) will block. This won't deadlock if no downstream code ever tries to acquire the baseline lock back, but it's a tight coupling between channel backpressure and adapter throughput.

**Fix:** Buffer events into a `Vec<RawUnlockEvent>` while holding the baseline lock, drop the lock, then drain the buffer with `tx.send`:

```rust
let mut events_to_send = Vec::new();
{
    let mut baseline = self.baseline.write().await;
    for (k, v) in state {
        let was = baseline.get(&k).copied().unwrap_or(false);
        if !was && v { events_to_send.push(make_evt(k, v)); }
        baseline.insert(k, v);
    }
} // baseline released
for evt in events_to_send { let _ = tx.send(evt).await; }
```

---

### WR-02: `read_object_body` silently accepts truncated VDF (missing 0x08 close tag)

**File:** `src-tauri/src/sources/vdf_binary.rs:81-87`

**Issue:** The loop uses `r.read(&mut tag)` (returns 0 on EOF) rather than `read_exact`. When EOF lands at the top of the loop, the function returns `Ok(Value::Object(map))` with the partial map. This means a truncated/malicious VDF that omits 0x08 close tags is silently accepted as if it were well-formed. The empirical fixtures all terminate properly so this never hits in practice — but T-31-T1 (Tampering UserGameStats) explicitly listed parser robustness as a mitigation, and the plan's W-2 acceptance asserts "non-zero leading byte returns Err" but does not assert "missing close tag returns Err".

For the *root* call (depth 0), accepting EOF is reasonable (some files end exactly at the last entry without an explicit 0x08). But for *nested* objects (depth > 0) the missing close tag indicates corruption.

**Fix:** Distinguish root vs nested EOF handling:

```rust
fn read_object_body<R: Read>(r: &mut R, depth: usize) -> anyhow::Result<Value> {
    if depth >= MAX_RECURSION_DEPTH { anyhow::bail!(...); }
    let mut map = HashMap::new();
    loop {
        let mut tag = [0u8; 1];
        let n = r.read(&mut tag)?;
        if n == 0 {
            if depth == 0 { break; }   // root EOF is benign
            anyhow::bail!("vdf_binary: unexpected EOF in nested object at depth {}", depth);
        }
        ...
    }
    Ok(Value::Object(map))
}
```

---

### WR-03: EnvGuard race — SC2 and SC3-supplement set the same env vars and run in parallel

**File:** `src-tauri/tests/integration_phase3.rs:236-260` and call sites

**Issue:** Cargo runs `#[tokio::test]` functions within the same integration-test binary in parallel by default (controlled by `RUST_TEST_THREADS`, default = num CPUs). Both `sc2_cream_api_and_sse_paths_auto_discovered` and `sc3_supplement_real_three_source_endtoend` set `HALLMARK_CREAMAPI_ROOT_OVERRIDE` and `HALLMARK_SSE_ROOT_OVERRIDE` to *different* fixture trees via EnvGuard. If they run concurrently:

1. SC2 sets cream root to `T1`.
2. SC3-supplement sets cream root to `T2`.
3. SC2 calls `discover_paths()` and reads `T2` (wrong tree) → asserts T1 path found → fails.
4. Or worse: SC2 EnvGuard drops, restores prev (None), removes the var while SC3-supplement is still using it → SC3-supplement reads default `%APPDATA%\CreamAPI` (the user's real machine) and silently passes/fails based on dev environment.

This will flake in CI on multi-core runners.

**Fix:** Either (a) gate both tests on a serial mutex via `serial_test` crate, (b) put both tests in `#[serial]` modules, or (c) use the `--test-threads=1` flag in CI for `integration_phase3`. Document the requirement in the test file's preamble.

---

### WR-04: SC3-supplement test silently tolerates dedup leak

**File:** `src-tauri/tests/integration_phase3.rs:694-717`

**Issue:** The drain loop only counts events as "extras" when `evt.ach_api_name == first.ach_api_name`. If the SSE adapter falls back to `<crc:0x...>` (because `load_goldberg_companion_keys` failed to write the companion in `%APPDATA%\GSE Saves\9999\` due to permissions or non-Windows test execution), it emits an event with a *different* ach_api_name. That event:

1. Gets through CrossSourceDedup (different key).
2. Gets persisted to SQLite (different row).
3. Is **not counted as an extra** by the test (line 700 filter on `first.ach_api_name`).
4. Is **not counted as a row** by the row-count assertion (line 723-724 filter on `first.ach_api_name`).

Net effect: the headline B-3 dedup property ("3 real adapters → 1 event") can fail silently as "2 events at sink, 2 rows in DB" and the test still passes. The plan's stated invariant — that the dedup invariant holds *because* the same CRC produces the same placeholder string deterministically — is not what the test actually asserts.

**Fix:** Count ALL events for `app_id == 9999` regardless of ach_api_name, and assert the total is 1:

```rust
// drain everything for app_id = 9999, not just matching ach_api_name
match timeout(remaining, sink_rx.recv()).await {
    Ok(Some(evt)) if evt.app_id == app_id => extras.push(evt),
    _ => break,
}
assert_eq!(extras.len(), 0, "dedup must collapse all 3 sources to 1 event");

// row count: count ALL rows for app_id, not filtered by api_name
let n: i64 = c.query_row("SELECT COUNT(*) FROM unlock_history WHERE app_id = ?1",
    rusqlite::params![app_id as i64], |r| r.get(0))?;
assert_eq!(n, 1);
```

Alternatively: assert that all three adapters resolve to the same ach_api_name *before* the pipeline fires (skip the test with `eprintln! + return` if the goldberg companion couldn't be written). The current "accept either form" wording in the plan does not match what dedup actually does.

---

### WR-05: SC1 latency assertion measures wrong thing

**File:** `src-tauri/tests/integration_phase3.rs:182-198`

**Issue:** The test claims to verify ROADMAP SC#1 ("Steam-legit unlock fires popup within 1s"). It measures `elapsed = t0.elapsed()` around the synchronous `adapter.on_file_changed(...).await` call, then asserts `elapsed < 1s`. But:

1. The file write (`std::fs::write`) at line 180 happens before t0, so file-write latency is excluded.
2. There's no debouncer in the path — `on_file_changed` is called directly. The 500ms debounce that production uses is bypassed.
3. `tx.send` happens inside `on_file_changed`, so when it returns the event is already enqueued. Measuring this is essentially measuring synchronous parse + emit time — single-digit milliseconds at most, never 1s.

The test will pass trivially (parse + emit takes ~1ms) regardless of whether the production pipeline meets the 1s SLA. The plan's B-1 fix rationale (avoid debouncer flakiness) is reasonable but the resulting test no longer verifies the documented SC#1 requirement.

**Fix:** Either (a) rename to `sc1_steam_legit_emits_event_synchronously` and drop the latency assertion (it's a smoke test, not a latency test), or (b) actually drive the debouncer end-to-end and accept the flakiness with a generous timeout.

---

### WR-06: Filename guards are case-sensitive on a case-insensitive filesystem

**Files:**
- `src-tauri/src/sources/cream_api.rs:210` — `Some("CreamAPI.Achievements.cfg")`
- `src-tauri/src/sources/sse.rs:309` — `Some("stats.bin")`
- `src-tauri/src/sources/steam_legit.rs:391, 395` — `starts_with("UserGameStatsSchema_")` etc.

**Issue:** Windows is case-insensitive at the filesystem level. `notify` typically returns the on-disk case, but if the source file was created with different case (e.g. user copy/paste, network share, backup tool), the name returned by `path.file_name()` may not match `STATE_FILENAME` exactly. CreamAPI's literal "CreamAPI.Achievements.cfg" is particularly mixed-case and brittle.

This is theoretical on a clean install but can manifest with unusual case in the wild. Goldberg avoided the issue by using all-lowercase filenames.

**Fix:** Use `eq_ignore_ascii_case`:

```rust
let matches = path.file_name()
    .and_then(|n| n.to_str())
    .map(|n| n.eq_ignore_ascii_case(STATE_FILENAME))
    .unwrap_or(false);
if !matches { return Ok(()); }
```

Same pattern in the steam_legit prefix checks (`starts_with` is case-sensitive in Rust).

---

### WR-07: `read_wstr_skip` reads pairs without alignment guard — adversarial WString can desync the parser

**File:** `src-tauri/src/sources/vdf_binary.rs:155-161`

**Issue:** The function reads 2 bytes at a time looking for `[0, 0]`. But if the malicious data has odd-byte alignment with a stray `0x00` byte that, paired with the next byte, makes a non-zero pair, the loop continues consuming bytes until a true `[0, 0]` is found — possibly far past the intended end of the WString. If the desync lands on a 0x08 close tag boundary, subsequent type-tag reads will misinterpret the byte stream.

In practice, achievement files don't contain WString tags so this code path is unreached, but the parser advertises support for tag 0x05.

**Fix:** Add a max-bytes safety bound similar to `read_cstr`'s 1024-byte cap:

```rust
fn read_wstr_skip<R: Read>(r: &mut R) -> anyhow::Result<()> {
    let mut consumed = 0usize;
    let mut pair = [0u8; 2];
    loop {
        r.read_exact(&mut pair)?;
        consumed += 2;
        if pair == [0, 0] { return Ok(()); }
        if consumed > 2048 {
            anyhow::bail!("vdf_binary: WString exceeds 2048 bytes");
        }
    }
}
```

---

### WR-08: `parse_creamapi_state` accepts empty section `[]` as a valid achievement key

**File:** `src-tauri/src/sources/cream_api.rs:109-116`

**Issue:** The check `line.starts_with('[') && line.ends_with(']') && line.len() >= 2` matches `[]` (length 2). `inner = &line[1..1]` = empty string. The empty string then becomes a HashMap key in the baseline: `out.insert("".to_string(), false)`. Subsequent `achieved=true` lines under that empty section flip it to true, and the adapter emits a `RawUnlockEvent { ach_api_name: "" }`.

A Cream INI file with `[]` blocks could fire empty-string achievement events that propagate to the popup queue and SQLite. SQLite UNIQUE INDEX would still dedup, but the popup display would be malformed.

**Fix:** Reject empty section names:

```rust
if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
    let inner = &line[1..line.len() - 1];
    if inner.trim().is_empty() { continue; }  // skip [   ] etc.
    ...
}
```

## Info

### IN-01: `parse_creamapi_state` does not handle `#`-prefixed lines as comments

**File:** `src-tauri/src/sources/cream_api.rs:106-108`

**Issue:** Documentation claims `###` triple-hash and `;` are comment markers. A line starting with a single `#` is treated as data (and silently ignored if it has no `=`, but parsed as `key=value` if it does). Other INI parsers commonly treat any leading `#` as a comment. If a CreamAPI variant in the wild uses single-hash comments, content like `# achieved=true` could be misparsed as a key.

**Fix:** Also strip lines beginning with `#` (single hash):

```rust
if line.starts_with('#') || line.starts_with(';') { continue; }
```

---

### IN-02: `log_discovery` uses inconsistent severity for "not detected" cases

**File:** `src-tauri/src/paths.rs:121-159`

**Issue:** Steam install absent → `tracing::warn!`. Goldberg roots+redirects empty → `tracing::warn!`. But Steam-legit appcache absent → `tracing::info!` (line 144). The Phase 3 path categories should match the Phase 1 severity convention — absent expected paths warn, present paths info.

**Fix:** Change line 144 from `tracing::info!` to `tracing::warn!` for the `None` branch of `steam_legit_appcache_stats`.

---

### IN-03: `MockAdapter` in `integration_phase3.rs` does no baseline tracking — duplicates aren't tested per-adapter

**File:** `src-tauri/tests/integration_phase3.rs:87-129`

**Issue:** The Phase 3 MockAdapter emits one event on every file event without tracking baseline/last_hash. Compared to the Phase 1 SC4 MockAdapter (which DOES baseline-track), this Phase 3 mock is weaker. The SC3 dedup test relies on this — the same payload triggers 3 emits → dedup catches them. But a regression in dedup would not be caught at the per-adapter level.

This is intentional per plan ("isolates dedup from per-adapter complexity") but worth flagging as a tradeoff.

---

### IN-04: SteamLegit `discover_paths` registry fallback enumerates `appcache_stats` files even when Steam is detected without registry users

**File:** `src-tauri/src/sources/steam_legit.rs:69-85`

**Issue:** The fallback `if user_ids.is_empty()` reads `appcache/stats` directory entries to extract user_ids from filenames. This is a sensible fallback when the registry has no `Users` subkey but Steam is otherwise installed. However, `steam_install` may be `None` (registry didn't return a SteamPath), in which case the fallback is skipped — but the function still returns `appcache_stats: None` and `user_ids: empty`. Nothing wrong with the logic, but the early-exit when `user_ids` is non-empty from registry skips the filename pass even if those registry user_ids don't match the actual `appcache/stats` filenames (e.g., user logged out, account removed). The `user_ids` filter in `on_file_changed` will then drop legitimate events.

**Fix:** Always merge filename-extracted user_ids with registry-extracted ones (union, not first-found-only):

```rust
// build user_ids from BOTH registry AND filename scan, then dedup
```

This adds robustness without changing the registry-first preference.

---

### IN-05: `parse_libraryfolders_text` silently accepts non-existent library paths

**File:** `src-tauri/src/paths.rs:280-294`

**Issue:** Library paths from `libraryfolders.vdf` are pushed without `path.exists()` validation. Downstream `scan_local_save_redirects` walks `<library>/steamapps/common/` and just skips the library if `common.exists()` returns false (line 448), so this is benign — but the discovered list logged at startup will include phantom libraries if VDF is stale. Mostly aesthetic.

**Fix:** Filter `libs` by `path.exists()` before returning:

```rust
libs.into_iter().filter(|p| p.exists()).collect()
```

---

### IN-06: SSE seed_baseline lock dance acquires/releases per-record needlessly

**File:** `src-tauri/src/sources/sse.rs:289-298`

**Issue:** The seed loop drops the baseline write lock, calls `resolve_api_name`, re-acquires the lock, inserts, drops again, and re-acquires for the next iteration. This is correct but expensive — N records → 2N+2 lock operations. Since `resolve_api_name` only acquires `crc_reverse` (a different lock), there's no actual conflict. Refactor to release baseline once, resolve all names into a Vec, then acquire once and bulk-insert.

**Fix:**

```rust
let mut to_insert: Vec<((u64, String), bool)> = Vec::new();
for rec in records {
    let api_name = resolve_api_name(&self.crc_reverse, app_id, &rec.crc32_hex).await;
    to_insert.push(((app_id, api_name), rec.achieved));
}
let mut baseline = self.baseline.write().await;
for (k, v) in to_insert { baseline.insert(k, v); total_entries += 1; }
```

---

## Fixes Applied

Fixed: 2026-05-09T11:30:00Z (10 of 10 critical+warning findings addressed; Info findings deferred per default scope).

| Finding | Commit  | Description |
| ------- | ------- | ----------- |
| CR-01 (cream_api.rs) | `c6942a1` | preserve baseline when send fails so retry can fire |
| CR-01 (steam_legit.rs) | `b1add45` | preserve baseline when send fails so retry can fire |
| CR-01 (sse.rs)       | `b740c4f` | preserve baseline when send fails so retry can fire |
| CR-02 (3 adapters)   | `da0e784` | close TOCTOU race on last_hash by claiming under single write lock |
| WR-01 (3 adapters)   | `cc2c41b` | release baseline lock before tx.send to avoid backpressure stalls |
| WR-02 (vdf_binary)   | `e083103` | reject EOF in nested VDF object as truncated/corrupt input |
| WR-03 (integration_phase3) | `ae40fb9` | serialise SC2 + SC3-supplement to avoid env-var race |
| WR-04 (integration_phase3) | `2c9e0bf` | SC3-supplement counts all events per app_id to catch dedup leaks |
| WR-05 (integration_phase3) | `51eb299` | rename SC1 to reflect synchronous emission scope, drop misleading latency assertion |
| WR-06 (3 adapters)   | `caa1e58` | case-insensitive filename guards on Windows filesystem |
| WR-07 (vdf_binary)   | `b26e083` | cap read_wstr_skip at 2048 bytes to prevent adversarial parser desync |
| WR-08 (cream_api)    | `0f9547c` | reject empty/whitespace-only section names in CreamAPI parser |

**Verification:** `cargo check --all-targets` passes; `cargo test --lib` runs 132 tests (baseline 131 + 1 new WR-08 test); `cargo test --test integration_phase1` runs 5; `cargo test --test integration_phase3` runs 5. All green.

**Info findings (IN-01 through IN-06)** were not in this fix scope (default = critical+warning). They remain documented above for follow-up.

---

_Reviewed: 2026-05-09T10:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
