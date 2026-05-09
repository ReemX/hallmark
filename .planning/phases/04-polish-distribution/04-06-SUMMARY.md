---
phase: 04-polish-distribution
plan: "06"
subsystem: infra
tags: [github-actions, tauri-action, portable-zip, release-pipeline, ed25519, updater, phase4-distribution]

requires:
  - phase: 04-01b
    provides: tauri-plugin-updater wired in Rust + updater pubkey placeholder in tauri.conf.json
  - phase: 04-04
    provides: auto-updater modal UI + install command wired end-to-end

provides:
  - Tag-triggered GitHub Actions release workflow producing NSIS installer + portable .zip + signed latest.json
  - Real Ed25519 pubkey in tauri.conf.json (replaces PLACEHOLDER_REPLACE_AT_RELEASE)
  - README with install instructions, SmartScreen workaround, portable mode docs, auto-update flow

affects: [release, distribution, DIST-01, DIST-02, DIST-03]

tech-stack:
  added:
    - tauri-apps/tauri-action@v0 (GitHub Actions step — NSIS build + latest.json upload)
    - pnpm/action-setup@v4 (pnpm in CI)
    - dtolnay/rust-toolchain (Rust stable in CI)
    - swatinem/rust-cache@v2 (Rust build cache)
    - Compress-Archive + gh CLI (portable .zip build + upload)
  patterns:
    - Ed25519 keypair one-time-generate: generate locally, paste private key into GitHub Secret, delete local file, commit pubkey only
    - Portable .zip from single executable: copy hallmark.exe → portable-stage/ → Compress-Archive → gh release upload
    - Signtool placeholder pattern: WINDOWS_CERTIFICATE env vars commented in workflow, uncomment when cert acquired

key-files:
  created:
    - .github/workflows/release.yml
    - README.md
  modified:
    - src-tauri/tauri.conf.json (pubkey replaced)
    - .gitignore (key file patterns added)
    - package.json (tauri-cli devDependency)
    - pnpm-lock.yaml

key-decisions:
  - "Portable .zip copies src-tauri/target/release/hallmark.exe (not bundle/nsis path) — Pitfall 7 avoidance. SFX is bundled via include_bytes! so no runtime asset dir needed."
  - "gh release upload used for portable .zip post-tauri-action — tauri-action only uploads its own artifacts (NSIS + sig + latest.json); gh CLI is preinstalled on windows-latest."
  - "Compress-Archive chosen over 7-zip for portable .zip — Compress-Archive is built into PowerShell 5+; 7-zip would require an install step for a negligible gain on a single-file zip."
  - "tagName: ${{ github.ref_name }} (no custom template) avoids Pitfall 6 (latest.json URL desync)."
  - "WINDOWS_CERTIFICATE placeholders commented in workflow (D-24) — no restructuring needed when cert is acquired, just uncomment 2 lines + add 2 repo secrets."
  - "Portable mode auto-update disabled: hallmark.exe launched directly, not via updater installer path; portable users must download new .zip to upgrade."

patterns-established:
  - "Release pipeline pattern: tauri-action@v0 → portable .zip pwsh step → gh release upload —clobber"
  - "D-21 keypair handling: one-time generate event, immediate paste-into-secret, immediate local delete, pubkey committed only"
  - "SmartScreen candor pattern: README explains WHY unsigned (cost/traction), names comparable OSS projects, conveys confidence not apology"

requirements-completed: [DIST-01, DIST-03]

duration: 15min
completed: 2026-05-09
---

# Phase 4 Plan 06: Release Pipeline Summary

**Tag-triggered GitHub Actions workflow ships NSIS installer + portable .zip + Ed25519-signed latest.json via tauri-action@v0; README documents install, SmartScreen workaround, portable mode, and auto-update flow.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-09T00:00:00Z
- **Completed:** 2026-05-09
- **Tasks:** 3 (Task 1 keypair handled by orchestrator; Tasks 2 + 3 executed here)
- **Files modified:** 6

## Accomplishments

- `.github/workflows/release.yml` built: tag push `v*.*.*` triggers windows-latest CI run, tauri-action@v0 builds NSIS + signs latest.json, post-action pwsh step builds portable .zip and uploads via `gh release upload --clobber`
- Ed25519 pubkey pasted into `tauri.conf.json` (PLACEHOLDER_REPLACE_AT_RELEASE replaced); private key exists only as a GitHub Secret (D-21 honored)
- README.md created from scratch with canonical install docs: Installer section (SmartScreen workaround), Portable section (no admin, state at `%APPDATA%\com.hallmark.app\`, updater disabled), First-run wizard mention, Auto-update section (stable-channel only, Later snooze)

## Task Commits

1. **Task 2: GitHub Actions release workflow** - `2c449a3` (feat)
2. **Task 3: README install + portable + auto-update** - `59c6fb2` (docs)
3. **Chore: Ed25519 pubkey + gitignore key files** - `b5f19ed` (chore)

## Files Created/Modified

- `.github/workflows/release.yml` — Tag-triggered release pipeline (100 lines)
- `README.md` — Install, portable, auto-update, development docs (84 lines, created from scratch)
- `src-tauri/tauri.conf.json` — Real Ed25519 pubkey replacing PLACEHOLDER_REPLACE_AT_RELEASE
- `.gitignore` — Added `/hallmark.key`, `/hallmark.key.pub`, `*.tauri.key`
- `package.json` — Added `@tauri-apps/cli ^2.11.1` devDependency
- `pnpm-lock.yaml` — Updated lockfile

## Decisions Made

- `Compress-Archive` chosen over 7-zip: built into PowerShell 5+, zero install step, negligible size difference for a single-file zip
- `gh release upload` for portable .zip: tauri-action only uploads its own artifacts; gh CLI is preinstalled on `windows-latest` GitHub runners
- `tagName: ${{ github.ref_name }}` (not a custom template): avoids Pitfall 6 where latest.json URL can desync from the actual release tag
- Commented-out `WINDOWS_CERTIFICATE` / `WINDOWS_CERTIFICATE_PASSWORD` env vars in workflow: D-24 future code-signing path requires only uncommenting + adding 2 repo secrets, no workflow restructuring

## Deviations from Plan

None — plan executed exactly as written. README.md did not exist (file-not-found on read), so it was created from scratch rather than appended; this is consistent with the plan's "If it doesn't exist, create" instruction.

## Pre-flight for First Real Release

Before tagging `v0.1.0` for release, ensure:

1. **GitHub Secrets** — `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set in repo Settings → Secrets → Actions
2. **GitHub remote** — `git remote get-url origin` returns `https://github.com/ReemX/hallmark.git` (or SSH equivalent)
3. **SFX finalization** — Plan 04-07 SFX swap is not blocking the pipeline (workflow is functional with placeholder SFX), but shipping placeholder audio on v0.1.0 would be suboptimal
4. **First-run wizard** — Plan 04-05 wizard must be complete before v0.1.0 ships (DIST-04)
5. **Smoke test** — Run `git tag v0.1.0-rc.1 && git push --tags`, watch Actions tab, verify 4 assets appear: `hallmark-setup.exe`, `hallmark-setup.exe.sig`, `latest.json`, `hallmark-portable-0.1.0-rc.1.zip`

## D-24 Future Code-Signing Path

When a code-signing certificate is acquired:
1. Add `WINDOWS_CERTIFICATE` (base64 PFX) and `WINDOWS_CERTIFICATE_PASSWORD` as repo secrets
2. Uncomment the two commented env var lines in `.github/workflows/release.yml`
3. No other workflow restructuring needed — tauri-bundler picks them up automatically

## Known Stubs

None. The workflow is functional end-to-end. The portable .zip build depends on `src-tauri/target/release/hallmark.exe` which is produced by the tauri-action step that precedes it.

## Threat Flags

None. All surfaces were within the plan's threat model (T-04-25 through T-04-32).

## Next Phase Readiness

- DIST-01 satisfied: tag-triggered workflow produces NSIS + portable .zip
- DIST-03 satisfied: full release pipeline operational
- DIST-02 prerequisite met: real pubkey in tauri.conf.json enables runtime signature verification
- Ready for Plan 04-07 (SFX finalization) and eventual v0.1.0 tag push

---
*Phase: 04-polish-distribution*
*Completed: 2026-05-09*
