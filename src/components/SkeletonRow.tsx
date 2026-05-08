export function SkeletonRow() {
  return (
    <div className="achievement-row skeleton" role="listitem" aria-hidden="true">
      <div className="row-icon"><div className="row-icon-placeholder" /></div>
      <div className="row-text">
        <div className="skeleton-line skeleton-title" />
        <div className="skeleton-line skeleton-desc" />
      </div>
    </div>
  );
}
