import { useState, type ReactNode } from "react";
import { Heart, MagnifyingGlass } from "@phosphor-icons/react";
import { EmptyState } from "../components/common/EmptyState";
import { TrackGridCard } from "../components/library/TrackGridCard";
import { NeteaseGridCard } from "../components/library/NeteaseGridCard";
import { VirtualGrid } from "../components/common/VirtualGrid";
import { formatDateTime } from "../lib/track";
import { useLibraryStore } from "../stores/libraryStore";

type FavEntry = {
  key: string;
  time: number;
  render: ReactNode;
};

export function FavoritesPage() {
  const favorites = useLibraryStore((state) => state.favorites);
  const neteaseFavorites = useLibraryStore((state) => state.neteaseFavorites);
  const removeFavorite = useLibraryStore((state) => state.removeFavorite);
  const removeNeteaseFavorite = useLibraryStore(
    (state) => state.removeNeteaseFavorite,
  );
  const [confirmKey, setConfirmKey] = useState<string | null>(null);

  const entries: FavEntry[] = [
    ...favorites.map((item) => ({
      key: `bili:${item.track.bvid}`,
      time: item.createdAt,
      render: (
        <TrackGridCard
          key={`bili:${item.track.bvid}`}
          track={item.track}
          subText={formatDateTime(item.createdAt)}
          removeLabel={
            confirmKey === `bili:${item.track.bvid}` ? "确认移除" : "移除"
          }
          onRemove={() => {
            if (confirmKey !== `bili:${item.track.bvid}`) {
              setConfirmKey(`bili:${item.track.bvid}`);
              return;
            }
            setConfirmKey(null);
            void removeFavorite(item.track.bvid, item.track.cid);
          }}
        />
      ),
    })),
    ...neteaseFavorites.map((item) => ({
      key: `netease:${item.track.songId}`,
      time: item.createdAt,
      render: (
        <NeteaseGridCard
          key={`netease:${item.track.songId}`}
          track={item.track}
          subText={formatDateTime(item.createdAt)}
          removeLabel={
            confirmKey === `netease:${item.track.songId}` ? "确认移除" : "移除"
          }
          onRemove={() => {
            if (confirmKey !== `netease:${item.track.songId}`) {
              setConfirmKey(`netease:${item.track.songId}`);
              return;
            }
            setConfirmKey(null);
            void removeNeteaseFavorite(item.track.songId);
          }}
        />
      ),
    })),
  ].sort((a, b) => b.time - a.time);

  return (
    <section className="page page--library">
      
      <h1>收藏</h1>
      {entries.length === 0 ? (
        <EmptyState
          icon={Heart}
          title="还没有收藏"
          hint="播放时点击心形即可收藏喜欢的曲目"
          action={
            <a className="button button--primary" href="#/search">
              <MagnifyingGlass size={16} aria-hidden />
              去搜索
            </a>
          }
        />
      ) : (
        <VirtualGrid
          items={entries}
          renderItem={(entry) => entry.render}
        />
      )}
    </section>
  );
}
