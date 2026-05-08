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
