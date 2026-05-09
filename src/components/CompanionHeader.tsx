import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function CompanionHeader({ gameName, sessionEarned }: { gameName: string; sessionEarned: number }) {
  return (
    <header className="companion-header" data-tauri-drag-region>
      <div className="companion-header-title" data-tauri-drag-region>{gameName}</div>
      {sessionEarned > 0 && (
        <div className="companion-header-badge" data-tauri-drag-region>{sessionEarned} earned this session</div>
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
