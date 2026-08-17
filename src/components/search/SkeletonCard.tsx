export function SkeletonCard() {
  return (
    <div className="search-card search-card--skeleton" aria-hidden>
      <span className="search-card__cover skeleton-block" />
      <span className="skeleton-line skeleton-line--title" />
      <span className="skeleton-line skeleton-line--meta" />
    </div>
  );
}
