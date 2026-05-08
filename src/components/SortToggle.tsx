type Sort = "earned-first" | "a-z";
export function SortToggle({ value, onChange }: { value: Sort; onChange: (v: Sort) => void }) {
  return (
    <div className="companion-sort" role="radiogroup" aria-label="Sort">
      <button
        type="button"
        role="radio"
        aria-checked={value === "earned-first"}
        className={`sort-chip ${value === "earned-first" ? "active" : ""}`}
        onClick={() => onChange("earned-first")}
      >
        Earned first
      </button>
      <button
        type="button"
        role="radio"
        aria-checked={value === "a-z"}
        className={`sort-chip ${value === "a-z" ? "active" : ""}`}
        onClick={() => onChange("a-z")}
      >
        A–Z
      </button>
    </div>
  );
}
