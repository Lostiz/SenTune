import { useState } from "react";
import { Heart, MagnifyingGlass } from "@phosphor-icons/react";
import { EmptyState } from "../components/common/EmptyState";
import { TrackGridCard } from "../components/library/TrackGridCard";
import { VirtualGrid } from "../components/common/VirtualGrid";
import { formatDateTime } from "../lib/track";
import { useLibraryStore } from "../stores/libraryStore";

export function FavoritesPage() {
  const favorites = useLibraryStore((state) => state.favorites);
  const removeFavorite = useLibraryStore((state) => state.removeFavorite);
  const [confirmBvid, setConfirmBvid] = useState<string | null>(null);

  return (
    <section className="page page--library">
      <p className="page__subtitle">资料库</p>
      <h1>收藏</h1>
      {favorites.length === 0 ? (
        <EmptyState
          icon={Heart}
          title="还没有收藏"
          hint="播放时点击心形即可收藏喜欢的视频"
          action={
            <a className="button button--primary" href="#/search">
              <MagnifyingGlass size={16} aria-hidden />
              去搜索
            </a>
          }
        />
      ) : (
        <VirtualGrid
          items={favorites}
          renderItem={(item) => (
            <TrackGridCard
              key={item.track.bvid}
              track={item.track}
              subText={formatDateTime(item.createdAt)}
              removeLabel={confirmBvid === item.track.bvid ? "确认移除" : "移除"}
              onRemove={() => {
                if (confirmBvid !== item.track.bvid) {
                  setConfirmBvid(item.track.bvid);
                  return;
                }
                setConfirmBvid(null);
                void removeFavorite(item.track.bvid, item.track.cid);
              }}
            />
          )}
        />
      )}
    </section>
  );
}
