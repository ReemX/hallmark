import type { SourceStatus } from "../types";

/**
 * Wizard variant of source row. UI-SPEC § Component Inventory authorizes
 * inheriting SettingsSourceRow's layout. The visible difference is purely
 * CSS via the parent .wizard-shell selector.
 */
export function WizardSourceRow({ source }: { source: SourceStatus }) {
  return (
    <div className={`source-row ${source.found ? "found" : "not-found"}`} role="listitem">
      <span className="source-mark" aria-hidden>{source.found ? "✓" : "✗"}</span>
      <span className="source-name">
        {source.found ? source.name : `${source.name}${source.detail ? ` — ${source.detail}` : " — not detected"}`}
      </span>
    </div>
  );
}
