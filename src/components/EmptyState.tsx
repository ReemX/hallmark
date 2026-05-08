type Variant = "no-game" | "loading" | "no-achievements" | "schema-failed";
const COPY: Record<Variant, { heading: string; body: string }> = {
  "no-game": { heading: "No game detected", body: "Start a Steam game to see achievements here." },
  "loading": { heading: "Loading achievements…", body: "Fetching achievement list for this game." },
  "no-achievements": { heading: "No achievements", body: "This game has no tracked achievements." },
  "schema-failed": { heading: "Couldn’t load achievements", body: "Check your internet connection or try relaunching the game." },
};
export function EmptyState({ variant }: { variant: Variant }) {
  const c = COPY[variant];
  return (
    <div className="companion-empty">
      <div className="empty-heading">{c.heading}</div>
      <div className="empty-body">{c.body}</div>
    </div>
  );
}
