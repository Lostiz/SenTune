import { Play, Plus } from "@phosphor-icons/react";
import { usePlayerStore } from "../../stores/playerStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { useToastStore } from "../../stores/toastStore";
import { formatDuration } from "../../lib/track";
import { CoverImage } from "../common/CoverImage";
import type { TrackInfo, VideoItem } from "../../types/models";

interface SearchCardProps {
  item: VideoItem;
  eager: boolean;
}

export function SearchCard({ item, eager }: SearchCardProps) {
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

  const handlePlay = () => {
    void playVideo(item);
    showToast(`正在播放：${item.title}`);
  };

  return (
    <div className="search-card-wrap">
      <button
        type="button"
        className="search-card"
        onClick={handlePlay}
        aria-label={`播放 ${item.title}，UP 主：${item.author}，时长 ${formatDuration(item.duration)}`}
      >
        <span className="search-card__cover">
          <CoverImage src={item.pic} alt="" eager={eager} />
          <span className="search-card__play" aria-hidden>
            <Play size={20} weight="fill" />
          </span>
          <span className="search-card__duration">
            {formatDuration(item.duration)}
          </span>
        </span>
        <span className="search-card__title" title={item.title}>
          {item.title}
        </span>
        <span className="search-card__meta">
          <span className="search-card__author" title={item.author}>
            {item.author}
          </span>
          <span className="search-card__play-count">
            {formatPlayCount(item.play)}
          </span>
        </span>
      </button>
      <button
        type="button"
        className="search-card__more"
        aria-label={`将「${item.title}」添加到歌单`}
        title="添加到歌单"
        onClick={() => openPicker(track)}
      >
        <Plus size={16} weight="bold" aria-hidden />
      </button>
    </div>
  );
}

function formatPlayCount(value: number): string {
  if (value >= 10_000) {
    return `${(value / 10_000).toFixed(value >= 100_000 ? 0 : 1)}万`;
  }
  return String(value);
}
