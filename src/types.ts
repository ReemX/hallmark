/** Mirrors Rust `popup_queue::PopupPayload` (serde). */
export type Tier = "standard" | "rare" | "completion";
export interface PopupPayload {
  app_id: number;
  ach_api_name: string;
  display_name: string;     // falls back to ach_api_name (D-26)
  description: string;
  icon_path: string | null; // absolute path to local icon file; null → placeholder
  global_pct: number | null;
  tier: Tier;
}

/** Mirrors Rust `schema::AchievementSchema` (serde). */
export interface AchievementSchema {
  app_id: number;
  ach_api_name: string;
  display_name: string | null;
  description: string | null;
  icon_path: string | null;
  hidden: boolean;
  global_pct: number | null;
}

/** Game-session events emitted from Rust game_detect task. */
export interface GameStartedPayload { app_id: number; }
export interface GameStoppedPayload {}
export interface SchemaResolvedPayload { app_id: number; }

/** Phase 4 — surfaces of DiscoveredPaths to Settings + Wizard React pages. */
export interface SourceStatus {
  name: "Steam" | "Goldberg" | "CreamAPI" | "SmartSteamEmu";
  found: boolean;
  detail?: string; // e.g. "libraryfolders.vdf not found"
}
export interface DiscoveredPathsView {
  sources: SourceStatus[];
}

/** Phase 4 — UpdateModal payload. Mirrors `tauri_plugin_updater::Update` subset. */
export interface UpdateInfo {
  version: string;
  notes: string | null;
}

/**
 * Tagged-enum return type from the `manual_check_update` Tauri command.
 * Mirrors the Rust `updater_glue::CheckOutcome` (serde tag = "status",
 * rename_all = "snake_case"). Phase 4 gap-closure 04-12.
 *
 * The `_yet` suffix on `no_release_yet` matches the Rust variant
 * `NoReleaseYet` after snake_case conversion.
 */
export type CheckOutcome =
  | { status: "available"; version: string; notes: string | null }
  | { status: "up_to_date" }
  | { status: "no_release_yet" }
  | { status: "offline"; detail: string }
  | { status: "platform_missing"; detail: string }
  | { status: "other_error"; detail: string };

/** Phase 4 — first-run wizard payload. */
export interface FirstRunState {
  sources: SourceStatus[];
  any_found: boolean;
}
