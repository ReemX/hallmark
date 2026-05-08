import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { GameStartedPayload, SchemaResolvedPayload } from "../types";

/** Listen to game-started / game-stopped / schema-resolved events from Rust. */
export function useGameSession() {
  const [appId, setAppId] = useState<number | null>(null);
  const [resolveStage, setResolveStage] = useState<string | null>(null);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return; // browser preview — Tauri APIs unavailable
    const u1 = listen<GameStartedPayload>("game-started", (e) => {
      setAppId(e.payload.app_id);
      setResolveStage(null);
    });
    const u2 = listen("game-stopped", () => {
      setAppId(null);
      setResolveStage(null);
    });
    const u3 = listen<SchemaResolvedPayload & { stage?: string }>("schema-resolved", (e) => {
      setResolveStage((e.payload as { stage?: string }).stage ?? "metadata");
    });
    return () => {
      u1.then((u) => u());
      u2.then((u) => u());
      u3.then((u) => u());
    };
  }, []);

  return { appId, resolveStage };
}
