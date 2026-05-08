import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function CompanionHeader({ gameName, sessionEarned }: { gameName: string; sessionEarned: number }) {
  return (
    <header className="companion-header" data-tauri-drag-region>
      <div className="companion-header-title">{gameName}</div>
      {sessionEarned > 0 && (
        <div className="companion-header-badge">{sessionEarned} earned this session</div>
      )}
      <button
        type="button"
        className="companion-close"
        aria-label="Close companion"
        onClick={() => getCurrentWebviewWindow().hide().catch(() => {})}
      >
        ×
      </button>
    </header>
  );
}
