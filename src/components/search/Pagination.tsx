import { CaretLeft, CaretRight } from "@phosphor-icons/react";

interface PaginationProps {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  disabled?: boolean;
}

function buildPages(page: number, totalPages: number): (number | "…")[] {
  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, index) => index + 1);
  }
  const pages: (number | "…")[] = [1];
  const start = Math.max(2, page - 1);
  const end = Math.min(totalPages - 1, page + 1);
  if (start > 2) pages.push("…");
  for (let current = start; current <= end; current += 1) {
    pages.push(current);
  }
  if (end < totalPages - 1) pages.push("…");
  pages.push(totalPages);
  return pages;
}

export function Pagination({
  page,
  totalPages,
  onPageChange,
  disabled = false,
}: PaginationProps) {
  if (totalPages <= 1) return null;

  return (
    <nav className="pagination" aria-label="搜索结果分页">
      <button
        type="button"
        className="pagination__button"
        aria-label="上一页"
        disabled={disabled || page <= 1}
        onClick={() => onPageChange(page - 1)}
      >
        <CaretLeft size={14} aria-hidden />
        上一页
      </button>
      <div className="pagination__pages">
        {buildPages(page, totalPages).map((entry, index) =>
          entry === "…" ? (
            <span key={`ellipsis-${index}`} className="pagination__ellipsis" aria-hidden>
              …
            </span>
          ) : (
            <button
              key={entry}
              type="button"
              className={`pagination__page${
                entry === page ? " pagination__page--current" : ""
              }`}
              aria-label={`第 ${entry} 页`}
              aria-current={entry === page ? "page" : undefined}
              disabled={disabled}
              onClick={() => onPageChange(entry)}
            >
              {entry}
            </button>
          ),
        )}
      </div>
      <button
        type="button"
        className="pagination__button"
        aria-label="下一页"
        disabled={disabled || page >= totalPages}
        onClick={() => onPageChange(page + 1)}
      >
        下一页
        <CaretRight size={14} aria-hidden />
      </button>
    </nav>
  );
}
