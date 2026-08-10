import { useState } from "react";
import { ClockCounterClockwise, Trash } from "@phosphor-icons/react";
import { EmptyState } from "../components/common/EmptyState";
import { TrackGridCard } from "../components/library/TrackGridCard";
import { VirtualGrid } from "../components/common/VirtualGrid";
import { formatDateTime } from "../lib/track";
import { useLibraryStore } from "../stores/libraryStore";

export function HistoryPage() {
  const history = useLibraryStore((state) => state.history);
  const clearHistory = useLibraryStore((state) => state.clearHistory);
  const [confirmClear, setConfirmClear] = useState(false);

  return (
    <section className="page page--library">
      <div className="page__header">
        <div>
          <p className="page__subtitle">资料库</p>
          <h1>历史</h1>
        </div>
        {history.length > 0 && (
          <button
            type="button"
            className={`button ${confirmClear ? "button--danger" : "button--ghost"}`}
            onClick={() => {
              if (!confirmClear) {
                setConfirmClear(true);
                return;
              }
              setConfirmClear(false);
              void clearHistory();
            }}
          >
            <Trash size={16} aria-hidden />
            {confirmClear ? "确认清空" : "清空历史"}
          </button>
        )}
      </div>
      {history.length === 0 ? (
        <EmptyState
          icon={ClockCounterClockwise}
          title="暂无播放历史"
          hint="播放过的视频会按时间倒序出现在这里"
        />
      ) : (
        <VirtualGrid
          items={history}
          renderItem={(item) => (
            <TrackGridCard
              key={`${item.track.bvid}-${item.playedAt}`}
              track={item.track}
              subText={formatDateTime(item.playedAt)}
            />
          )}
        />
      )}
    </section>
  );
}
