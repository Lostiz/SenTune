import { useState, type ReactNode } from "react";
import { ClockCounterClockwise, Trash } from "@phosphor-icons/react";
import { EmptyState } from "../components/common/EmptyState";
import { TrackGridCard } from "../components/library/TrackGridCard";
import { NeteaseGridCard } from "../components/library/NeteaseGridCard";
import { VirtualGrid } from "../components/common/VirtualGrid";
import { formatDateTime } from "../lib/track";
import { useLibraryStore } from "../stores/libraryStore";

type HistoryEntry = {
  key: string;
  time: number;
  render: ReactNode;
};

export function HistoryPage() {
  const history = useLibraryStore((state) => state.history);
  const neteaseHistory = useLibraryStore((state) => state.neteaseHistory);
  const clearHistory = useLibraryStore((state) => state.clearHistory);
  const clearNeteaseHistory = useLibraryStore(
    (state) => state.clearNeteaseHistory,
  );
  const [confirmClear, setConfirmClear] = useState(false);

  const entries: HistoryEntry[] = [
    ...history.map((item) => ({
      key: `bili:${item.track.bvid}`,
      time: item.playedAt,
      render: (
        <TrackGridCard
          key={`bili:${item.track.bvid}`}
          track={item.track}
          subText={formatDateTime(item.playedAt)}
        />
      ),
    })),
    ...neteaseHistory.map((item) => ({
      key: `netease:${item.track.songId}`,
      time: item.playedAt,
      render: (
        <NeteaseGridCard
          key={`netease:${item.track.songId}`}
          track={item.track}
          subText={formatDateTime(item.playedAt)}
        />
      ),
    })),
  ].sort((a, b) => b.time - a.time);

  return (
    <section className="page page--library">
      <div className="page__header">
        <div>
          
          <h1>历史</h1>
        </div>
        {entries.length > 0 && (
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
              void clearNeteaseHistory();
            }}
          >
            <Trash size={16} aria-hidden />
            {confirmClear ? "确认清空" : "清空历史"}
          </button>
        )}
      </div>
      {entries.length === 0 ? (
        <EmptyState
          icon={ClockCounterClockwise}
          title="暂无播放历史"
          hint="播放过的曲目会按时间倒序出现在这里"
        />
      ) : (
        <VirtualGrid items={entries} renderItem={(entry) => entry.render} />
      )}
    </section>
  );
}
