import type { SourceStatus } from "../types";

export function SettingsSourceRow({ source }: { source: SourceStatus }) {
  return (
    <div className={`source-row ${source.found ? "found" : "not-found"}`} role="listitem">
      <span className="source-mark" aria-hidden>{source.found ? "✓" : "✗"}</span>
      <span className="source-name">
        {source.found
          ? source.name
          : `${source.name}${source.detail ? ` — ${source.detail}` : " — not detected"}`}
      </span>
    </div>
  );
}
