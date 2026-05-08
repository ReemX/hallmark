//! Phase 2 schema resolution chain (D-24): SQLite cache → local Steam appcache
//! → Goldberg achievements.json metadata → public Steam Web API rarity →
//! cache-back. Plan 02 owns this module.
//!
//! Plan 01 stub — submodule declarations only. Plan 02 populates the
//! orchestrator (SchemaCache), AchievementSchema, classify_tier.

pub mod cache;
pub mod appcache;
pub mod steam_api;
pub mod goldberg_meta;
