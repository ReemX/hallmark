import { useEffect, useState, useCallback } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useGameSession } from "./hooks/useGameSession";
import { CompanionHeader } from "./components/CompanionHeader";
import { FilterBar } from "./components/FilterBar";
import { SortToggle } from "./components/SortToggle";
import { AchievementRow } from "./components/AchievementRow";
import { SkeletonRow } from "./components/SkeletonRow";
import { EmptyState } from "./components/EmptyState";
import "./styles/companion.css";
import type { AchievementSchema } from "./types";

interface CompanionState {
  app_id: number;
  schema: AchievementSchema[];
  earned: Record<string, number>;
  session_id: string;
}
interface CompanionPrefs {
  app_id: number;
  filter: "all" | "earned" | "locked" | null;
  sort: "earned-first" | "a-z" | null;
  expanded_id: string | null;
  width: number | null;
  height: number | null;
  pos_x: number | null;
  pos_y: number | null;
}

function CompanionRoot() {
  const { appId, resolveStage } = useGameSession();
  const [state, setState] = useState<CompanionState | null>(null);
  const [prefs, setPrefs] = useState<CompanionPrefs | null>(null);
  const [error, setError] = useState<string | null>(null);

  // ----- D-17: show/hide on game-start/stop -----
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return; // browser preview — Tauri APIs unavailable
    const w = getCurrentWebviewWindow();
    if (appId !== null) {
      w.show().catch(() => {});
      setError(null);
      // Fetch state + prefs in parallel.
      Promise.all([
        invoke<CompanionState>("get_companion_state", { app_id: appId }),
        invoke<CompanionPrefs | null>("get_companion_prefs_cmd", { app_id: appId }),
      ])
      .then(([s, p]) => {
        setState(s);
        setPrefs(p ?? { app_id: appId, filter: "all", sort: "earned-first", expanded_id: null, width: null, height: null, pos_x: null, pos_y: null });
      })
      .catch((e) => setError(String(e)));
    } else {
      w.hide().catch(() => {});
      setState(null);
      setPrefs(null);
    }
  }, [appId]);

  // ----- D-20: refetch on schema-resolved (in-place upgrade) -----
  useEffect(() => {
    if (resolveStage && appId !== null) {
      invoke<CompanionState>("get_companion_state", { app_id: appId })
        .then(setState)
        .catch((e) => setError(String(e)));
    }
  }, [resolveStage, appId]);

  // ----- D-18: filter/sort/expand persistence (debounced 500ms) -----
  const persistPrefs = useDebouncedPersist(prefs);

  const filter = prefs?.filter ?? "all";
  const sort = prefs?.sort ?? "earned-first";
  const expandedId = prefs?.expanded_id ?? null;

  const onFilterChange = useCallback((f: "all" | "earned" | "locked") => {
    setPrefs((p) => p ? { ...p, filter: f } : p);
  }, []);
  const onSortChange = useCallback((s: "earned-first" | "a-z") => {
    setPrefs((p) => p ? { ...p, sort: s } : p);
  }, []);
  const onToggleExpand = useCallback((api_name: string) => {
    setPrefs((p) => p ? { ...p, expanded_id: p.expanded_id === api_name ? null : api_name } : p);
  }, []);

  useEffect(() => { persistPrefs(); }, [prefs, persistPrefs]);

  // ----- Render -----
  if (error) {
    return (
      <div className="companion-shell">
        <CompanionHeader gameName="Hallmark" sessionEarned={0} />
        <EmptyState variant="schema-failed" />
      </div>
    );
  }
  if (appId === null) {
    return (
      <div className="companion-shell">
        <CompanionHeader gameName="Hallmark" sessionEarned={0} />
        <EmptyState variant="no-game" />
      </div>
    );
  }
  if (!state || !prefs) {
    return (
      <div className="companion-shell">
        <CompanionHeader gameName="Loading game…" sessionEarned={0} />
        <div className="companion-list" role="list">
          {Array.from({ length: 6 }).map((_, i) => <SkeletonRow key={i} />)}
        </div>
      </div>
    );
  }
  if (state.schema.length === 0) {
    return (
      <div className="companion-shell">
        <CompanionHeader gameName="Loading game…" sessionEarned={Object.keys(state.earned).length} />
        <EmptyState variant={resolveStage ? "no-achievements" : "loading"} />
      </div>
    );
  }

  // ----- Filter + sort the schema list -----
  const earnedSet = new Set(Object.keys(state.earned));
  const filtered = state.schema.filter((a) => {
    if (filter === "earned") return earnedSet.has(a.ach_api_name);
    if (filter === "locked") return !earnedSet.has(a.ach_api_name);
    return true;
  });
  const sorted = [...filtered].sort((a, b) => {
    if (sort === "a-z") {
      return (a.display_name ?? a.ach_api_name).localeCompare(b.display_name ?? b.ach_api_name);
    }
    // earned-first: earned rows before locked; tiebreak by api_name asc.
    const ae = earnedSet.has(a.ach_api_name) ? 0 : 1;
    const be = earnedSet.has(b.ach_api_name) ? 0 : 1;
    if (ae !== be) return ae - be;
    return (a.display_name ?? a.ach_api_name).localeCompare(b.display_name ?? b.ach_api_name);
  });

  return (
    <div className="companion-shell">
      <CompanionHeader
        gameName={`App ${state.app_id}`}
        sessionEarned={Object.keys(state.earned).length}
      />
      <div className="companion-controls">
        <FilterBar value={filter} onChange={onFilterChange} />
        <SortToggle value={sort} onChange={onSortChange} />
      </div>
      <div className="companion-list" role="list">
        {sorted.map((a) => (
          <AchievementRow
            key={a.ach_api_name}
            ach={a}
            earnedAt={state.earned[a.ach_api_name] ?? null}
            expanded={expandedId === a.ach_api_name}
            onToggle={() => onToggleExpand(a.ach_api_name)}
          />
        ))}
      </div>
    </div>
  );
}

/** Debounced persist hook — fires set_companion_prefs_cmd 500ms after last change. */
function useDebouncedPersist(prefs: CompanionPrefs | null) {
  return useCallback(() => {
    if (!prefs) return;
    const handle = window.setTimeout(() => {
      invoke("set_companion_prefs_cmd", { prefs }).catch(() => {});
    }, 500);
    return () => window.clearTimeout(handle);
  }, [prefs]);
}

const root = document.getElementById("root");
if (root) createRoot(root).render(<CompanionRoot />);
