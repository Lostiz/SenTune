import { Play, Plus, X } from "@phosphor-icons/react";
import { usePlayerStore } from "../../stores/playerStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { formatDuration, trackToQueueItem } from "../../lib/track";
import { CoverImage } from "../common/CoverImage";
import type { TrackInfo } from "../../types/models";

interface TrackGridCardProps {
  track: TrackInfo;
  subText?: string;
  onRemove?: () => void;
  removeLabel?: string;
}

export function TrackGridCard({
  track,
  subText,
  onRemove,
  removeLabel = "移除",
}: TrackGridCardProps) {
  const playItem = usePlayerStore((state) => state.playItem);
  const openPicker = usePlaylistPickerStore((state) => state.openPicker);

  return (
    <div className="search-card-wrap">
      <button
        type="button"
        className="search-card"
        onClick={() => playItem(trackToQueueItem(track))}
        aria-label={`播放 ${track.title}，UP 主：${track.author}`}
      >
        <span className="search-card__cover">
          <CoverImage src={track.coverUrl} alt="" />
          <span className="search-card__play" aria-hidden>
            <Play size={20} weight="fill" />
          </span>
          <span className="search-card__duration">
            {formatDuration(track.duration)}
          </span>
          {track.cachedAt !== null && (
            <span className="search-card__badge">已缓存</span>
          )}
        </span>
        <span className="search-card__title" title={track.title}>
          {track.title}
        </span>
        <span className="search-card__meta">
          <span className="search-card__author" title={track.author}>
            {track.author}
          </span>
          {subText && <span className="search-card__sub">{subText}</span>}
        </span>
      </button>
      <button
        type="button"
        className="search-card__more"
        aria-label={`将「${track.title}」添加到歌单`}
        title="添加到歌单"
        onClick={() => openPicker(track)}
      >
        <Plus size={16} weight="bold" aria-hidden />
      </button>
      {onRemove && (
        <button
          type="button"
          className="search-card__remove"
          aria-label={`${removeLabel}「${track.title}」`}
          title={removeLabel}
          onClick={onRemove}
        >
          <X size={14} weight="bold" aria-hidden />
        </button>
      )}
    </div>
  );
}
