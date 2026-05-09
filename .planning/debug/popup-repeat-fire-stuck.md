---
status: diagnosed
trigger: "popup-repeat-fire-stuck: first 'Fire test popup' tray click renders the popup correctly. Every subsequent test_trigger::fire — even >60 s later, well past the 10 s CrossSourceDedup TTL — logs only test_trigger::fire but produces NO downstream UNLOCK line and NO popup_queue: POPUP_FIRED line. Popup never re-renders."
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — second+ fires are silently dropped by the SQLite UNIQUE INDEX dedup inside `run_pipeline`, NOT by the in-memory CrossSourceDedup TTL.
test: read-only static analysis of test_trigger::fire → run_pipeline → record_unlock → unlock_history schema
expecting: confirm path leads from raw_tx through dedup (10s TTL — passes >10s later) into record_unlock with INSERT OR IGNORE on UNIQUE INDEX (app_id, ach_api_name, session_id), which returns rows_changed=0 → `Ok(false)` → run_pipeline `continue` at line 393 of watcher/mod.rs without logging UNLOCK
next_action: return diagnosis to caller

## Symptoms

expected: Every tray "Fire test popup" click renders a popup (with the 10 s dedup TTL only suppressing within-window repeats). Logs should show test_trigger::fire → watcher: UNLOCK app_id=480 → popup_queue: POPUP_FIRED for each fire that is past the dedup window.
actual: |
  16:52:50 test_trigger: test popup fired ... app_id=480 api_name="HALLMARK_TEST_UNLOCK"
  16:52:50 watcher: UNLOCK app_id=480 ach=HALLMARK_TEST_UNLOCK source=goldberg
  16:52:50 popup_queue: POPUP_FIRED app_id=480 ach=HALLMARK_TEST_UNLOCK tier="standard" depth_after=0
  16:53:53 test_trigger: test popup fired ... app_id=480  (no UNLOCK, no POPUP_FIRED)
  16:53:54 test_trigger: test popup fired ...           (no UNLOCK, no POPUP_FIRED)
  16:53:56 test_trigger: test popup fired ...           (no UNLOCK, no POPUP_FIRED)
errors: None — no panic, no error log between fires. (Dedup-drop logs in run_pipeline are tracing::debug, suppressed by default RUST_LOG=hallmark_lib=info,warn.)
reproduction: cargo tauri dev → tray "Fire test popup" → wait >10s → tray "Fire test popup" again → no popup.
started: Phase 4 UAT test 4 on 2026-05-09. Code paths added in Phase 4 plans 04-01b and 04-03.

## Eliminated

- hypothesis: H1 — popup_queue's select! drain consumes the receiver and exits after first popup hide
  evidence: popup_queue::run is a `loop { tokio::select! { ... } }` that returns ONLY on `sink_rx.recv() == None` (sink closed). After processing a single event, it sleeps 3300ms then 500ms, then loops back — does not exit. And the symptom shows NO `popup_queue: POPUP_FIRED` AND NO `UNLOCK` for the second fire — so the issue is upstream of popup_queue. (popup_queue/mod.rs:81-110)
  timestamp: 2026-05-09

- hypothesis: H2 — run_pipeline's mpsc receiver drops or breaks after first event
  evidence: run_pipeline is `while let Some(evt) = raw_rx.recv().await { ... }` — only returns when raw_tx is dropped/closed. AppState holds a clone of raw_tx for the process lifetime, so the channel cannot close while the app is running. Furthermore the second fire's `test_trigger: test popup fired` log line is emitted ONLY after `raw_tx.blocking_send(evt)` returns Ok — proving the channel is alive and the event was accepted into the buffer. (test_trigger.rs:51-58, watcher/mod.rs:359)
  timestamp: 2026-05-09

- hypothesis: H3 — popup window state is broken after first hide
  evidence: would manifest as POPUP_FIRED log present but no visual popup. Symptom shows NO POPUP_FIRED log for second fire — issue is upstream of popup_queue.
  timestamp: 2026-05-09

- hypothesis: H4 — CrossSourceDedup persists entries past TTL
  evidence: dedup.rs:48-61 — `is_duplicate` calls `self.seen.retain(|_, ts| now.duration_since(*ts) < ttl)` BEFORE the contains_key check. Tested by `expired_observation_is_no_longer_duplicate` (dedup.rs:95-101). After >10s the entry is swept, second fire would NOT be flagged as duplicate. (Symptom timeline 16:52:50 → 16:53:53 = 63s, well past TTL.)
  timestamp: 2026-05-09

- hypothesis: H5 — Frontend popup window listener unsubscribes after first auto-dismiss
  evidence: would manifest as POPUP_FIRED log present but visual popup missing. Symptom shows NO POPUP_FIRED for second fire — issue is upstream.
  timestamp: 2026-05-09

## Evidence

- timestamp: 2026-05-09
  checked: src-tauri/src/test_trigger.rs::fire (lines 32-65)
  found: After raw_tx.blocking_send returns Ok, logs "test popup fired (synthetic event injected at adapter→dedup boundary)". Failure path logs warn "test_trigger send failed (channel closed?)". Symptom shows the success log every time — proving the send succeeded and the event entered the channel. Channel is mpsc with capacity 64; not full.
  implication: First piece of pipeline (test_trigger → raw_tx) is healthy on every fire.

- timestamp: 2026-05-09
  checked: src-tauri/src/watcher/mod.rs::run_pipeline (lines 350-409)
  found: Loop is `while let Some(evt) = raw_rx.recv().await`. For each event:
    1. Lock dedup, call `is_duplicate(evt.app_id, &evt.ach_api_name)`.
    2. If is_dup: tracing::debug!(...) + `continue;` (skips UNLOCK log, skips sink.send).
    3. Else: store.record_unlock(...) → returns `inserted: bool`.
    4. If !inserted: tracing::debug!(...) + `continue;` (also skips UNLOCK log, skips sink.send).
    5. Only when inserted: tracing::info!("UNLOCK") and sink.send(evt).
  implication: TWO silent-drop paths exist. Both log only at debug level, invisible under default RUST_LOG. Symptom (no UNLOCK log) is consistent with EITHER path firing.

- timestamp: 2026-05-09
  checked: src-tauri/src/watcher/dedup.rs::CrossSourceDedup::is_duplicate (lines 48-61)
  found: Sweeps expired entries (now.duration_since(ts) >= ttl) before checking. Test `expired_observation_is_no_longer_duplicate` proves the sweep works at the boundary. With ttl=10s and second fire at ~63s, the first observation is swept and the second fire is NOT a duplicate.
  implication: CrossSourceDedup is NOT the gate that drops the second fire. The drop must happen in `record_unlock`.

- timestamp: 2026-05-09
  checked: src-tauri/src/store/mod.rs::record_unlock (lines 58-84) + src-tauri/src/store/migrations/001_initial.sql (lines 24-25)
  found: `INSERT OR IGNORE INTO unlock_history (app_id, ach_api_name, source, unlocked_at, session_id, notified) VALUES (...)`. The unique index `idx_unlock_dedup` is on `(app_id, ach_api_name, session_id)`. INSERT OR IGNORE returns rows_changed=0 on collision; record_unlock returns `Ok(false)`.
  implication: Once a row is inserted in unlock_history with (480, HALLMARK_TEST_UNLOCK, session_id), every subsequent `record_unlock` call with the same triplet returns `Ok(false)` for the lifetime of the session. session_id is stable for the entire `cargo tauri dev` run.

- timestamp: 2026-05-09
  checked: src-tauri/src/lib.rs::run (lines 286-289)
  found: `let session_id = uuid::Uuid::new_v4().to_string();` — assigned ONCE at startup. Stored in AppState. NEVER rotated mid-process.
  implication: Confirms session_id stays constant for every test_trigger::fire in one app run. Therefore (480, HALLMARK_TEST_UNLOCK, session_id) collides on every repeat fire.

- timestamp: 2026-05-09
  checked: src-tauri/src/lib.rs::init_tracing (lines 213-224)
  found: Default RUST_LOG filter is "hallmark_lib=info,warn". The two silent-drop tracing::debug! calls in run_pipeline are SUPPRESSED at this level.
  implication: Explains why the user sees zero log output between the test_trigger::fire log and the missing UNLOCK log — the drop happens at debug level, default filter hides it. (Setting RUST_LOG=hallmark_lib=debug would reveal `DB-level dedup: row already existed (UNIQUE INDEX)`.)

- timestamp: 2026-05-09
  checked: 04-03-SUMMARY.md line 139 (T-04-13 threat)
  found: "T-04-13 (rapid clicks): accepted — CrossSourceDedup 10s TTL governs."
  implication: The plan author considered ONLY the in-memory 10s TTL as the gate for repeat fires. The DB-level `UNIQUE INDEX (app_id, ach_api_name, session_id)` is permanent within a session — it was overlooked as a longer-lived gate that affects test_trigger because the test re-uses the same (app_id, ach_api_name) pair every fire. The DB UNIQUE INDEX is correctly sized for REAL achievements (a real Spacewar achievement would only unlock once per session and that's correct), but the synthetic test fires the SAME (480, HALLMARK_TEST_UNLOCK) pair repeatedly — colliding on the DB index forever.

## Resolution

root_cause: |
  The "Fire test popup" tray menu uses a fixed (app_id=480, ach_api_name="HALLMARK_TEST_UNLOCK") pair. The first fire records that triplet in `unlock_history` against the current session_id. Every subsequent fire — regardless of how long the user waits — passes the in-memory 10s CrossSourceDedup (which expires after 10s), then hits the SQLite UNIQUE INDEX `idx_unlock_dedup ON unlock_history(app_id, ach_api_name, session_id)`. INSERT OR IGNORE returns rows_changed=0; record_unlock returns Ok(false); run_pipeline silently `continue`s without logging UNLOCK and without forwarding to sink_tx — so popup_queue never sees the event and never fires the popup.

  This is a design oversight in Phase 4 Plan 03: T-04-13 documented "CrossSourceDedup 10s TTL governs" repeat-click behavior, but the second dedup layer (DB UNIQUE INDEX, scoped to session lifetime) was not considered. The DB UNIQUE INDEX is correct for REAL game achievements (where re-firing the same achievement in one session IS a duplicate to suppress), but is wrong for the synthetic test trigger that intentionally re-uses the same key on every click.

  Both silent-drop paths in run_pipeline log only at tracing::debug level, which is below the default RUST_LOG threshold (info). This is why the symptom shows zero log evidence between `test_trigger: test popup fired` and the missing `UNLOCK` line.

fix: (not applied — diagnose-only mode)
verification: (n/a — diagnose-only mode)
files_changed: []
