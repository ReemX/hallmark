import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { SettingsSourceRow } from "./components/SettingsSourceRow";
import type { DiscoveredPathsView, UpdateInfo } from "./types";

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
      },
      {
        name: "Goldberg",
        found: d.goldberg_save_roots.length > 0,
        detail: d.goldberg_save_roots.length === 0 ? "saves directory not found" : undefined,
      },
      {
        name: "CreamAPI",
        found: d.cream_api_appid_dirs.length > 0,
        detail: d.cream_api_appid_dirs.length === 0 ? "no per-game directories found" : undefined,
      },
      {
        name: "SmartSteamEmu",
        found: d.sse_appid_dirs.length > 0,
        detail: d.sse_appid_dirs.length === 0 ? "saves directory not found" : undefined,
      },
    ],
  };
}

function relativeTime(unixSecs: number | null): string {
  if (unixSecs === null) return "never";
  const ageSecs = Math.floor(Date.now() / 1000) - unixSecs;
  if (ageSecs < 60) return "just now";
  if (ageSecs < 3600) return `${Math.floor(ageSecs / 60)} min ago`;
  if (ageSecs < 86400) return `${Math.floor(ageSecs / 3600)} hr ago`;
  return `${Math.floor(ageSecs / 86400)} days ago`;
}

type UpdateState =
  | { kind: "idle"; lastChecked: number | null }
  | { kind: "checking" }
  | { kind: "uptodate"; lastChecked: number }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "error"; message: string };

function SettingsRoot() {
  const [view, setView] = useState<DiscoveredPathsView | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);
  const [updateState, setUpdateState] = useState<UpdateState>({ kind: "idle", lastChecked: null });

  const VERSION = "0.1.0"; // mirrors src-tauri/Cargo.toml workspace version

  // Initial scan — load cached_discovery via rescan_paths command.
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    // Phase 4 gap closure (04-09): signal backend that settings WebView mounted.
    invoke("settings_ready").catch((e) => console.warn("settings_ready invoke failed:", e));

    invoke<DiscoveredPathsRust>("rescan_paths")
      .then((d) => setView(rustToView(d)))
      .catch((e) => setScanError(String(e)));
  }, []);

  const handleRescan = useCallback(async () => {
    setScanning(true);
    setScanError(null);
    try {
      const d = await invoke<DiscoveredPathsRust>("rescan_paths");
      setView(rustToView(d));
    } catch (e) {
      setScanError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  const handleCheckUpdates = useCallback(async () => {
    setUpdateState({ kind: "checking" });
    try {
      const result = await invoke<UpdateInfo | null>("manual_check_update");
      if (result) {
        setUpdateState({ kind: "available", info: result });
      } else {
        setUpdateState({ kind: "uptodate", lastChecked: Math.floor(Date.now() / 1000) });
      }
    } catch (e) {
      setUpdateState({ kind: "error", message: String(e) });
    }
  }, []);

  const handleInstall = useCallback(async () => {
    try {
      await invoke("install_pending_update");
    } catch (e) {
      setUpdateState({ kind: "error", message: String(e) });
    }
  }, []);

  const handleClose = useCallback(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    getCurrentWebviewWindow().close().catch(() => {});
  }, []);

  return (
    <div className="settings-shell">
      <div className="settings-header" data-tauri-drag-region>
        <span className="settings-title">Settings</span>
        <button
          className="settings-close"
          onClick={handleClose}
          aria-label="Close"
        >
          ×
        </button>
      </div>

      <div className="settings-body">

        {/* Detected Sources */}
        <section className="settings-section">
          <h2 className="settings-section-heading">Detected Sources</h2>
          <p className="settings-section-intro">Achievement sources found on this system:</p>
          {scanning ? (
            <div className="settings-source-list" role="list">
              {[0, 1, 2].map((i) => <div key={i} className="skeleton-line" />)}
            </div>
          ) : view ? (
            <div className="settings-source-list" role="list">
              {view.sources.map((s) => (
                <SettingsSourceRow key={s.name} source={s} />
              ))}
            </div>
          ) : (
            <p className="settings-error">{scanError ?? "Loading…"}</p>
          )}
          <button className="settings-pill-button" onClick={handleRescan} disabled={scanning}>
            {scanning ? "Scanning…" : "Rescan"}
          </button>
          {scanError && !scanning && (
            <p className="settings-error">Something went wrong during rescan. Try again.</p>
          )}
        </section>

        <hr className="settings-divider" />

        {/* Updates */}
        <section className="settings-section">
          <h2 className="settings-section-heading">Updates</h2>
          <p className="settings-section-intro">Hallmark v{VERSION}</p>
          {updateState.kind === "idle" && (
            <p className="settings-meta">Last checked: never</p>
          )}
          {updateState.kind === "uptodate" && (
            <>
              <p className="settings-status">You&apos;re on the latest version.</p>
              <p className="settings-meta">Last checked: {relativeTime(updateState.lastChecked)}</p>
            </>
          )}
          {updateState.kind === "available" && (
            <p className="settings-status">
              Version <span className="settings-accent">{updateState.info.version}</span> is available.
            </p>
          )}
          {updateState.kind === "checking" && (
            <p className="settings-meta">Checking for updates…</p>
          )}
          {updateState.kind === "error" && (
            <p className="settings-error">Couldn&apos;t reach the update server. Check your connection.</p>
          )}
          {updateState.kind === "available" ? (
            <button className="settings-pill-button" onClick={handleInstall}>
              Install and Restart Hallmark
            </button>
          ) : (
            <button
              className="settings-pill-button"
              onClick={handleCheckUpdates}
              disabled={updateState.kind === "checking"}
            >
              {updateState.kind === "error" ? "Retry" : "Check for Updates"}
            </button>
          )}
        </section>

        <hr className="settings-divider" />

        {/* About */}
        <section className="settings-section">
          <h2 className="settings-section-heading">About</h2>
          <p className="settings-meta">Hallmark v{VERSION}</p>
          <p className="settings-meta">
            <a
              className="settings-link"
              href="https://github.com/ReemX/hallmark"
              target="_blank"
              rel="noreferrer noopener"
            >
              View on GitHub
            </a>
          </p>
          <p className="settings-meta">MIT License</p>
        </section>

      </div>
    </div>
  );
}

export default SettingsRoot;
