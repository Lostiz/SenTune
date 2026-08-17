import { Plus } from "@phosphor-icons/react";
import { usePlayerStore } from "../../stores/playerStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { useToastStore } from "../../stores/toastStore";
import { formatDuration } from "../../lib/track";
import { CoverImage } from "../common/CoverImage";
import type { TrackInfo, VideoItem } from "../../types/models";

interface SearchRowProps {
  item: VideoItem;
  eager?: boolean;
}

function formatPlayCount(value: number): string {
  if (value >= 10_000) {
    return `${(value / 10_000).toFixed(value >= 100_000 ? 0 : 1)}万`;
  }
  return String(value);
}

export function SearchRow({ item, eager = false }: SearchRowProps) {
  const playVideo = usePlayerStore((state) => state.playVideo);
  const openPicker = usePlaylistPickerStore((state) => state.openPicker);
  const showToast = useToastStore((state) => state.showToast);

  const track: TrackInfo = {
    bvid: item.bvid,
    cid: 0,
    title: item.title,
    coverUrl: item.pic,
    author: item.author,
    duration: item.duration,
    audioId: 0,
    codec: "",
    cachePath: null,
    cachedAt: null,
    lastPlayedAt: null,
    playCount: 0,
  };

  return (
    <div className="search-row-wrap">
      <button
        type="button"
        className="search-row"
        onClick={() => {
          void playVideo(item);
          showToast(`正在播放：${item.title}`);
        }}
        aria-label={`播放 ${item.title}，UP 主：${item.author}`}
      >
        <CoverImage
          src={item.pic}
          alt=""
          className="search-row__cover"
          eager={eager}
          width={96}
          height={96}
        />
        <span className="search-row__info">
          <span className="search-row__title" title={item.title}>
            {item.title}
          </span>
          <span className="search-row__meta">
            {item.author} · {formatDuration(item.duration)} ·{" "}
            {formatPlayCount(item.play)} 播放
          </span>
        </span>
      </button>
      <button
        type="button"
        className="search-row__more"
        aria-label={`将「${item.title}」添加到歌单`}
        title="添加到歌单"
        onClick={() => openPicker(track)}
      >
        <Plus size={16} weight="bold" aria-hidden />
      </button>
    </div>
  );
}
