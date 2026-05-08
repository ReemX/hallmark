-- Phase 1 schema: unlock detection persistence.
-- Phase 2 will add schema_cache + icon_cache; Phase 3 may extend sessions.
-- This file is loaded via include_str! at compile time and applied idempotently
-- on every SqliteStore::open(). All statements use IF NOT EXISTS for restart safety.

CREATE TABLE IF NOT EXISTS unlock_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    app_id        INTEGER NOT NULL,
    ach_api_name  TEXT    NOT NULL,
    source        TEXT    NOT NULL,
    unlocked_at   INTEGER NOT NULL,
    -- WR-11: session_id is NOT NULL. SQLite treats NULL as distinct from NULL
    -- in UNIQUE INDEX, so allowing NULL silently disabled the dedup constraint
    -- whenever a bug elsewhere dropped the session_id. Production code always
    -- passes a session id (Plan 05); the schema now enforces that.
    session_id    TEXT    NOT NULL,
    notified      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_unlock_session ON unlock_history(session_id);
CREATE INDEX IF NOT EXISTS idx_unlock_app     ON unlock_history(app_id, ach_api_name);
-- Belt-and-suspenders dedup: cross-source dedup TTL (Plan 05) is the primary
-- mechanism; this UNIQUE INDEX catches anything the in-memory dedup misses
-- (e.g. process restart mid-session). REQ DETECT-07.
CREATE UNIQUE INDEX IF NOT EXISTS idx_unlock_dedup
    ON unlock_history(app_id, ach_api_name, session_id);

CREATE TABLE IF NOT EXISTS sessions (
    session_id    TEXT    PRIMARY KEY,
    app_id        INTEGER,
    started_at    INTEGER NOT NULL,
    ended_at      INTEGER
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
