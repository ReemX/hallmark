import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WizardSourceRow } from "./components/WizardSourceRow";
import type { DiscoveredPathsView, SourceStatus } from "./types";

interface DiscoveredPathsRust {
  steam_install: string | null;
  steam_libraries: string[];
  goldberg_save_roots: string[];
  goldberg_local_save_redirects: { target_path: string; app_id: number }[];
  steam_legit_appcache_stats: string | null;
  steam_legit_user_ids: number[];
  cream_api_appid_dirs: string[];
  sse_appid_dirs: string[];
}

function rustToView(d: DiscoveredPathsRust): DiscoveredPathsView {
  return {
    sources: [
      {
        name: "Steam",
        found: !!d.steam_legit_appcache_stats,
        detail: !d.steam_legit_appcache_stats ? "libraryfolders.vdf not found" : undefined,
      } as SourceStatus,
      {
        name: "Goldberg",
        found: d.goldberg_save_roots.length > 0,
        detail: d.goldberg_save_roots.length === 0 ? "saves directory not found" : undefined,
      } as SourceStatus,
      {
        name: "CreamAPI",
        found: d.cream_api_appid_dirs.length > 0,
        detail: d.cream_api_appid_dirs.length === 0 ? "no per-game directories found" : undefined,
      } as SourceStatus,
      {
        name: "SmartSteamEmu",
        found: d.sse_appid_dirs.length > 0,
        detail: d.sse_appid_dirs.length === 0 ? "saves directory not found" : undefined,
      } as SourceStatus,
    ],
  };
}

function FirstRunWizardRoot() {
  const [view, setView] = useState<DiscoveredPathsView | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dismissing, setDismissing] = useState(false);

  // Initial scan on mount.
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    // Phase 4 gap closure (04-09): signal backend that wizard WebView mounted.
    // Currently a no-op for behavior (no backend emits to wizard) but provides
    // an instrumentation log line marking 'wizard React tree mounted' and
    // future-proofs the surface in case a backend task ever needs to emit here.
    invoke("wizard_ready").catch((e) => console.warn("wizard_ready invoke failed:", e));

    invoke<DiscoveredPathsRust>("rescan_paths")
      .then((d) => setView(rustToView(d)))
      .catch((e) => setError(String(e)));
  }, []);

  const handleRescan = useCallback(async () => {
    setScanning(true);
    setError(null);
    try {
      const d = await invoke<DiscoveredPathsRust>("rescan_paths");
      setView(rustToView(d));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  const handleDismiss = useCallback(async () => {
    setDismissing(true);
    try {
      // wizard_dismiss writes first_run_done IFF any path was found in cached_discovery.
      // It also closes the wizard window programmatically.
      await invoke("wizard_dismiss");
    } catch (e) {
      setError(String(e));
      setDismissing(false);
    }
  }, []);

  // Loading state — initial scan in flight, no view yet.
  if (!view && !error) {
    return (
      <div className="wizard-shell">
        <div className="wizard-header" data-tauri-drag-region>
          <span className="wizard-title">Welcome to Hallmark</span>
        </div>
        <div className="wizard-body">
          <div className="settings-source-list" role="list">
            {[0, 1, 2, 3].map((i) => <div key={i} className="skeleton-line" />)}
          </div>
        </div>
      </div>
    );
  }

  const anyFound = view?.sources.some((s) => s.found) ?? false;

  return (
    <div className="wizard-shell">
      <div className="wizard-header" data-tauri-drag-region>
        <span className="wizard-title">{anyFound ? "Welcome to Hallmark" : "No sources detected yet"}</span>
      </div>
      <div className="wizard-body">

        {anyFound ? (
          // N > 0: D-15 happy path
          <>
            <p className="wizard-subheading">We found these achievement sources on your system:</p>
            <div className="settings-source-list" role="list">
              {view!.sources.filter((s) => s.found).map((s) => (
                <WizardSourceRow key={s.name} source={s} />
              ))}
            </div>
          </>
        ) : (
          // N = 0: D-16 honest framing
          <>
            <p className="wizard-subheading">Here's what we looked for:</p>
            <div className="settings-source-list" role="list">
              {view!.sources.map((s) => (<WizardSourceRow key={s.name} source={s} />))}
            </div>
            <p className="wizard-explainer">
              Hallmark watches these locations automatically. Install Steam or launch a game to populate them.
            </p>
            {error && <p className="settings-error">Scan failed. You can continue and rescan from Settings later.</p>}
          </>
        )}

        <div className="wizard-buttons">
          {!anyFound && (
            <button
              className="settings-pill-button"
              onClick={handleRescan}
              disabled={scanning || dismissing}
            >
              {scanning ? "Scanning…" : "Rescan"}
            </button>
          )}
          <button
            className="wizard-cta-primary"
            onClick={handleDismiss}
            disabled={scanning || dismissing}
          >
            {dismissing ? "…" : "Get started"}
          </button>
          {!anyFound && (
            <button
              className="wizard-cta-secondary"
              onClick={handleDismiss}
              disabled={scanning || dismissing}
            >
              Continue anyway
            </button>
          )}
        </div>

      </div>
    </div>
  );
}

export default FirstRunWizardRoot;
