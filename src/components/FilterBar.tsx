type Filter = "all" | "earned" | "locked";
export function FilterBar({ value, onChange }: { value: Filter; onChange: (v: Filter) => void }) {
  const opts: { v: Filter; label: string }[] = [
    { v: "all", label: "All" },
    { v: "earned", label: "Earned" },
    { v: "locked", label: "Locked" },
  ];
  return (
    <div className="companion-filter" role="radiogroup" aria-label="Filter">
      {opts.map((o) => (
        <button
          key={o.v}
          type="button"
          role="radio"
          aria-checked={value === o.v}
          className={`filter-chip ${value === o.v ? "active" : ""}`}
          onClick={() => onChange(o.v)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
