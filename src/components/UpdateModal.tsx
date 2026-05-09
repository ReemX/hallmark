import { useState } from "react";
import { motion } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
// Phase 4 gap closure (04-11): shell.open routes the release-notes link
// through the Windows default browser. WebView2 blocks plain target="_blank".
import { open as openExternal } from "@tauri-apps/plugin-shell";
import type { UpdateInfo } from "../types";

interface Props {
  info: UpdateInfo;
  onDismiss: () => void;
}

const FADE_SCALE_IN = { duration: 0.2, ease: "easeOut" as const };
const FADE_SCALE_OUT = { duration: 0.15, ease: "easeIn" as const };

export function UpdateModal({ info, onDismiss }: Props) {
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleInstall = async () => {
    setInstalling(true);
    setError(null);
    try {
      await invoke("install_pending_update");
      // app.restart() never returns — this line is unreachable in practice.
    } catch (e) {
      setError(String(e));
      setInstalling(false);
    }
  };

  // Truncate notes to ~280 chars (UI-SPEC: "Release notes (truncated)")
  const truncated =
    info.notes && info.notes.length > 280
      ? info.notes.slice(0, 280) + "…"
      : (info.notes ?? "");

  return (
    <motion.div
      className="update-modal-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={FADE_SCALE_OUT}
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-modal-heading"
    >
      <motion.div
        className="update-modal-card"
        initial={{ opacity: 0, scale: 0.96 }}
        animate={{ opacity: 1, scale: 1.0 }}
        exit={{ opacity: 0, scale: 0.96 }}
        transition={FADE_SCALE_IN}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="update-modal-heading" className="update-modal-heading">
          Update available
        </h2>
        <span className="update-modal-version">v{info.version}</span>
        {truncated && (
          <>
            <p className="update-modal-notes-label">What&apos;s new:</p>
            <p className="update-modal-notes">{truncated}</p>
          </>
        )}
        <a
          className="update-modal-link"
          href={`https://github.com/ReemX/hallmark/releases/tag/v${info.version}`}
          onClick={(e) => {
            e.preventDefault();
            // Silently swallow rejection — capability mismatch should not
            // crash the modal. Right-click "Copy link" remains as fallback.
            openExternal(`https://github.com/ReemX/hallmark/releases/tag/v${info.version}`).catch(() => {});
          }}
          target="_blank"
          rel="noreferrer noopener"
        >
          Read full release notes on GitHub
        </a>
        {error && <p className="update-modal-error">{error}</p>}
        <div className="update-modal-buttons">
          <button
            className="update-modal-install"
            onClick={handleInstall}
            disabled={installing}
          >
            {installing ? "Installing…" : "Install and Restart Hallmark"}
          </button>
          <button
            className="update-modal-snooze"
            onClick={onDismiss}
            disabled={installing}
          >
            Later
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
