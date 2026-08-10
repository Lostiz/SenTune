import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

interface VirtualGridProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  minColWidth?: number;
  gap?: number;
  overscan?: number;
  onNearBottom?: () => void;
  footer?: ReactNode;
  className?: string;
}

/** 定高网格虚拟化：仅渲染可视行，适合超过 200 项的封面墙。 */
export function VirtualGrid<T>({
  items,
  renderItem,
  minColWidth = 160,
  gap = 16,
  overscan = 2,
  onNearBottom,
  footer,
  className = "",
}: VirtualGridProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({
    width: 0,
    height: 0,
    scrollTop: 0,
  });

  useLayoutEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const update = () =>
      setSize({
        width: element.clientWidth,
        height: element.clientHeight,
        scrollTop: element.scrollTop,
      });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    element.addEventListener("scroll", update, { passive: true });
    return () => {
      observer.disconnect();
      element.removeEventListener("scroll", update);
    };
  }, []);

  const columns = Math.max(1, Math.floor((size.width + gap) / (minColWidth + gap)));
  const colWidth = minColWidth;
  const rowHeight = Math.round(colWidth) + 74;
  const rows = Math.max(0, Math.ceil(items.length / columns));
  const startRow = Math.max(
    0,
    Math.floor(size.scrollTop / (rowHeight + gap)) - overscan,
  );
  const endRow = Math.min(
    rows,
    Math.ceil((size.scrollTop + size.height) / (rowHeight + gap)) + overscan,
  );

  const cells: ReactNode[] = [];
  for (let row = startRow; row < endRow; row += 1) {
    for (let col = 0; col < columns; col += 1) {
      const index = row * columns + col;
      if (index >= items.length) break;
      cells.push(
        <div
          key={index}
          className="virtual-grid__cell"
          style={{
            top: row * (rowHeight + gap),
            left: col * (colWidth + gap),
            width: colWidth,
            height: rowHeight,
          }}
        >
          {renderItem(items[index], index)}
        </div>,
      );
    }
  }

  const totalHeight = rows > 0 ? rows * (rowHeight + gap) - gap : 0;

  useEffect(() => {
    if (!onNearBottom || rows === 0 || size.height === 0) return;
    if (size.scrollTop + size.height >= totalHeight - 400) {
      onNearBottom();
    }
  }, [size, rows, totalHeight, onNearBottom]);

  return (
    <div
      ref={containerRef}
      className={`virtual-grid ${className}`.trim()}
      role="list"
    >
      <div className="virtual-grid__spacer" style={{ height: totalHeight }} />
      {cells}
      {footer && <div className="virtual-grid__footer">{footer}</div>}
    </div>
  );
}
