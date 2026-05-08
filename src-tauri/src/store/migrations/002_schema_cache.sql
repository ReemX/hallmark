-- Phase 2 schema additions: achievement schema cache + companion window preferences.
-- Loaded via include_str! at compile time and applied idempotently after 001_initial.sql
-- on every SqliteStore::open() call. All statements use IF NOT EXISTS for restart safety.
--
-- 100%-completion flags re-use the existing settings table (key = 'completion_<app_id>',
-- value = '1') per CONTEXT.md D-11. No new table needed for that.

-- Achievement schema cache. One row per (app_id, ach_api_name).
-- display_name / description / icon_path / global_pct are nullable so a partial
-- resolution (rarity-without-name from Web API alone) can still be cached.
-- icon_path is an absolute filesystem path to a downloaded image in the user data dir;
-- storing path (not BLOB) keeps row reads cheap and lets WebView2 load the file
-- via convertFileSrc() without a SQLite roundtrip.
CREATE TABLE IF NOT EXISTS schema_cache (
    app_id          INTEGER NOT NULL,
    ach_api_name    TEXT    NOT NULL,
    display_name    TEXT,
    description     TEXT,
    icon_path       TEXT,
    hidden          INTEGER NOT NULL DEFAULT 0,
    global_pct      REAL,
    cached_at       INTEGER NOT NULL,
    PRIMARY KEY (app_id, ach_api_name)
);
CREATE INDEX IF NOT EXISTS idx_schema_app ON schema_cache(app_id);

-- Companion window per-game preferences. Persists D-18 filter+sort state and
-- D-15 size+position (after first user move). One row per app_id.
CREATE TABLE IF NOT EXISTS companion_prefs (
    app_id      INTEGER PRIMARY KEY,
    filter      TEXT,        -- 'all' | 'earned' | 'locked'
    sort        TEXT,        -- 'earned-first' | 'a-z'
    expanded_id TEXT,        -- last-expanded ach_api_name, or NULL
    width       INTEGER,
    height      INTEGER,
    pos_x       INTEGER,
    pos_y       INTEGER
);
