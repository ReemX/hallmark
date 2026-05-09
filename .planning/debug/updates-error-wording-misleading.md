---
status: diagnosed
trigger: "In Settings → Updates panel, clicking 'Check for updates' displays 'Couldn't reach the update server. Check your connection.' But the user has working internet. Root cause is that GitHub Releases latest.json returns 404 because no v0.1.x has been published yet — different from a network failure."
created: 2026-05-09T00:00:00Z
updated: 2026-05-09T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED. Two-place bug.
  (1) `tauri-plugin-updater` 2.10 returns `Error::ReleaseNotFound` (Display: "Could not fetch a valid release JSON from the remote") whenever the endpoint responded with any non-2xx status — INCLUDING 404 — because the loop at updater.rs:481-518 silently logs and falls through on non-success, then line 528 unwraps via `Error::ReleaseNotFound`. This variant collapses "no release published" with "any other non-success response code".
  (2) `updater_glue::manual_check` flattens this to `e.to_string()`, so the frontend receives only the string.
  (3) `Settings.tsx` ignores `message` and hardcodes "Couldn't reach the update server. Check your connection." for any error.
test: complete — see Evidence section
expecting: complete
next_action: return ROOT CAUSE FOUND with minimal-fix recommendation

## Symptoms

expected: Error message is accurate to the actual cause. For a 404 (no release exists yet), either say "No releases available yet" or treat as the implicit "uptodate" state. For genuine network failure, "check your connection" is correct.
actual: Both 404 and offline produce "Couldn't reach the update server. Check your connection." Misleading the user.
errors: |
  ERROR tauri_plugin_updater::updater: update endpoint did not respond with a successful status code
  WARN hallmark_lib::updater_glue: update check failed error=Could not fetch a valid release JSON from the remote
reproduction: cargo tauri dev → tray → Settings → Updates → "Check for updates". Repo has no published releases yet at github.com/ReemX/hallmark/releases.
started: Discovered during Phase 4 UAT test 9 on 2026-05-09.

## Eliminated

- hypothesis: "tauri-plugin-updater 2.10's Error type doesn't carry enough info to distinguish 404 from network failure"
  evidence: |
    The Error enum (error.rs) clearly separates `ReleaseNotFound` (returned for any non-2xx response code from the endpoint, including 404) from `Reqwest(reqwest::Error)` (returned only for transport-layer failures: DNS, TCP connect, TLS, timeout). This is exactly the distinction we need. The hypothesis was a misread of the symptom — info IS available; the app just discards it via `e.to_string()`.
  timestamp: 2026-05-09T00:00:00Z

## Resolution

root_cause: |
  Two-place bug, both contributing.
  (1) **Backend lossy mapping** — `src-tauri/src/updater_glue.rs::manual_check` (line 61) and `lib.rs::manual_check_update` command (lines 164-169) coerce `tauri_plugin_updater::Error` to `String` via `e.to_string()`, throwing away the variant tag. The Error enum DOES distinguish `ReleaseNotFound` (any non-2xx HTTP, e.g. our 404 from a not-yet-published GitHub Release) from `Reqwest(reqwest::Error)` (genuine network/DNS/TLS failure), but that distinction is destroyed at the FFI boundary.
  (2) **Frontend hardcoded copy** — `src/Settings.tsx` line 180-182 ignores the `message` field of its `UpdateState.error` variant and renders the literal string `"Couldn't reach the update server. Check your connection."` for ANY error returned from `manual_check_update`. Even if the backend sent precise text, the frontend would not show it.
  Combined effect: a 404 from `https://github.com/ReemX/hallmark/releases/latest/download/latest.json` (which doesn't exist yet because no release is published) produces `Error::ReleaseNotFound`, gets stringified, and is rendered as the generic offline copy — actively misinforming the user.

fix: empty until applied
verification: empty until verified
files_changed: []

## Evidence

- timestamp: 2026-05-09T00:00:00Z
  checked: src/Settings.tsx Updates section render path
  found: |
    UpdateState union has a single error variant `{ kind: "error"; message: string }`. Render at lines 180-182:
      {updateState.kind === "error" && (
        <p className="settings-error">Couldn&apos;t reach the update server. Check your connection.</p>
      )}
    The `message` field on the error state is NOT used in the rendered output — the copy is hardcoded. So even if the backend sent a structured/specific message, the frontend would still display the generic "check your connection" string.
  implication: Bug is on BOTH sides: (a) backend flattens distinct errors into one stringly-typed result, AND (b) frontend ignores the message and renders a hardcoded sentence. Two-place fix required.

- timestamp: 2026-05-09T00:00:00Z
  checked: src-tauri/src/updater_glue.rs manual_check function
  found: |
    Returns Result<Option<UpdateInfoView>, String>. The String is just `e.to_string()` where `e: tauri_plugin_updater::Error`. No structured kind/category preserved. The lib.rs command wrapper (lines 164-169) just forwards this to the frontend.
  implication: Backend currently has no way to tell the frontend which kind of failure happened. Need to introduce a structured result type (e.g. an enum-tagged variant or a `{kind, message}` struct) that the frontend can switch on.

- timestamp: 2026-05-09T00:00:00Z
  checked: tauri-plugin-updater 2.10 backend log line
  found: |
    "ERROR tauri_plugin_updater::updater: update endpoint did not respond with a successful status code"
    paired with our own "WARN ... error=Could not fetch a valid release JSON from the remote".
    The plugin's own log line shows it CAN distinguish "non-success status code" — meaning the plugin internally received an HTTP response and noted it was non-2xx. So the request was NOT a network failure; it was a 404. The information exists at the plugin layer; the question is whether it's surfaced through the public Error API.
  implication: Root cause class is "unsuccessful HTTP status from endpoint" — a 404 — not "network unreachable". Frontend copy is therefore actively wrong, not just imprecise.

- timestamp: 2026-05-09T00:00:00Z
  checked: ~/.cargo/registry/src/index.crates.io-*/tauri-plugin-updater-2.10.1/src/error.rs (full enum)
  found: |
    `pub enum Error` is `#[non_exhaustive]` and includes these check-relevant variants:
      - EmptyEndpoints (no endpoints configured)
      - Io(std::io::Error)
      - Semver, Serialization, Base64, Minisign, SignatureUtf8 (data parsing)
      - ReleaseNotFound — "Could not fetch a valid release JSON from the remote"   <-- our case
      - UrlParse, Http, InvalidHeaderValue, InvalidHeaderName (URL/header construction)
      - Reqwest(reqwest::Error) — wraps the underlying request layer; reqwest::Error has
        is_connect(), is_timeout(), is_request(), is_decode(), and .status() -> Option<StatusCode>
      - TargetNotFound(String), TargetsNotFound(Vec<String>) (release JSON did not advertise the platform)
      - InsecureTransportProtocol (endpoint not https)
      - Network(String) — used ONLY in the download path (line 690: failed download HTTP status), NOT during check
      - Tauri(tauri::Error)
    The Error enum is matchable from the consumer side, and `#[from]` impls preserve the underlying type info (so `Reqwest(reqwest::Error)` retains `.status()` and `.is_connect()` for fine-grained branching).
  implication: Plenty of structured info available. The app just chose to flatten everything to `to_string()`.

- timestamp: 2026-05-09T00:00:00Z
  checked: tauri-plugin-updater 2.10 updater.rs:474-528 (the check() URL loop)
  found: |
    ```
    let response = request.build()?.get(url).headers(...).send().await;
    match response {
        Ok(res) => {
            if res.status().is_success() {            // 200/204 → parse or "no update"
                if StatusCode::NO_CONTENT == res.status() { return Ok(None); }
                let update_response: serde_json::Value = res.json().await?;
                ...
            } else {
                log::error!("update endpoint did not respond with a successful status code");
                // <-- DOES NOT set last_error. Loops to next endpoint.
            }
        }
        Err(err) => {
            log::error!("failed to check for updates: {err}");
            last_error = Some(err.into());           // network/DNS failure → Error::Reqwest
        }
    }
    // ... after the loop:
    if let Some(error) = last_error { return Err(error); }
    let release = remote_release.ok_or(Error::ReleaseNotFound)?;
    ```
    This is the smoking gun. Behavior table:
      | Scenario                        | Path taken                       | Surfaced Error                       |
      |---------------------------------|----------------------------------|--------------------------------------|
      | DNS/TCP failure (offline)       | `Err(err)` arm                   | `Error::Reqwest(_)` — has is_connect()|
      | Connection times out            | `Err(err)` arm                   | `Error::Reqwest(_)` — has is_timeout()|
      | TLS/cert failure                | `Err(err)` arm                   | `Error::Reqwest(_)`                  |
      | 404 / 403 / 5xx HTTP response   | `Ok(res)` arm but !is_success    | `Error::ReleaseNotFound`             |
      | 200 with malformed JSON         | json::from_value Err             | `Error::Serialization(_)`            |
      | Release missing this platform   | post-parse                       | `Error::TargetNotFound(_)` or TargetsNotFound|
      | Endpoint list empty             | pre-loop                         | `Error::EmptyEndpoints`              |
  implication: |
    Distinguishing 404 from offline is straightforward at the Rust layer:
      - matches!(e, Error::ReleaseNotFound)            → 404 (or any non-2xx HTTP) — "no release available yet"
      - matches!(e, Error::Reqwest(_))                  → genuine network/DNS/TLS failure — "check connection"
      - matches!(e, Error::TargetNotFound(_) | Error::TargetsNotFound(_)) → release exists but not for this platform
      - matches!(e, Error::EmptyEndpoints)              → config bug (Cargo.toml missing endpoints)
      - everything else                                  → fall through to "Update check failed: {message}"
    The Error::ReleaseNotFound variant is slightly imprecise (it conflates "endpoint returned 404" with "endpoint returned 500" with "endpoint returned malformed JSON that we couldn't recover from") — but for a v1 hobby project that hasn't shipped a release yet, treating ALL of those as "no release available yet" is correct UX. The Reqwest variant cleanly distinguishes the legitimately offline case.

- timestamp: 2026-05-09T00:00:00Z
  checked: minimal-fix shape design
  found: |
    Two changes, both small:
    Backend (src-tauri/src/updater_glue.rs + lib.rs commands):
      1. Replace the `Result<Option<UpdateInfoView>, String>` return with a tagged enum or struct that carries an error kind:
         ```rust
         #[derive(Serialize)]
         #[serde(tag = "status", rename_all = "snake_case")]
         pub enum CheckOutcome {
             Available { version: String, notes: Option<String> },
             UpToDate,
             NoReleaseYet,           // Error::ReleaseNotFound — most likely until v0.1.0 is published
             Offline { detail: String },  // Error::Reqwest(_)
             PlatformMissing { detail: String }, // Error::TargetNotFound | TargetsNotFound
             OtherError { detail: String },
         }
         ```
         Match on `tauri_plugin_updater::Error` variants in `manual_check` to map. Command return type becomes `Result<CheckOutcome, String>` (the outer `Err` reserved only for unrecoverable bugs like updater plugin not installed).
      2. Apply the same mapping in `spawn_background_check` so logs distinguish the cases (info!-level for NoReleaseYet, warn!-level for Offline).
    Frontend (src/Settings.tsx):
      1. Extend the `UpdateState` discriminated union with `{ kind: "no_release" }` and `{ kind: "offline" }` (and rename current `error` to a catch-all `other_error`).
      2. Pattern-match on `result.status` from the new tagged response and pick correct copy:
         - "no_release" → "No releases yet — Hallmark is on its first version. We'll show new versions here when they arrive." (and treat as success: persist last_checked, show "Last checked: just now")
         - "offline" → keep the current "Couldn't reach the update server. Check your connection." copy
         - "platform_missing" → "An update was found but doesn't support your platform." (rare)
         - "other_error" → "Update check failed: {detail}" (use the message, not a hardcoded string)
    Single MR can land both. No DB schema changes; `last_update_check` is already persisted via `persist_last_checked`. For `NoReleaseYet`, call `persist_last_checked` so user sees freshness.
  implication: |
    Recommended Plan-04 follow-up of ~30 lines Rust + ~25 lines TSX. Tests:
      - Unit test `manual_check` mapping by injecting a mock Error (the variants are pub).
      - Manual UAT: run cargo tauri dev with no published release → expect "No releases yet" copy. Disconnect WiFi → expect "Check your connection".
    Specialist hint: typescript (frontend copy + state) + rust (Error variant matching). Primary effort is Rust pattern-matching; mark as `rust`.
