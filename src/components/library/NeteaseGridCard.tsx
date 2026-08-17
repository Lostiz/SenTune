import { Play, X } from "@phosphor-icons/react";
import { usePlayerStore } from "../../stores/playerStore";
import { formatDuration } from "../../lib/track";
import { CoverImage } from "../common/CoverImage";
import type { NeteaseTrackInfo } from "../../types/models";

interface NeteaseGridCardProps {
  track: NeteaseTrackInfo;
  subText?: string;
  onRemove?: () => void;
  removeLabel?: string;
}

export function NeteaseGridCard({
  track,
  subText,
  onRemove,
  removeLabel = "移除",
}: NeteaseGridCardProps) {
  const playNeteaseTrack = usePlayerStore((state) => state.playNeteaseTrack);

  return (
    <div className="search-card-wrap">
      <button
        type="button"
        className="search-card"
        onClick={() => void playNeteaseTrack(track)}
        aria-label={`播放 ${track.title}，歌手：${track.artist}`}
      >
        <span className="search-card__cover">
          <CoverImage src={track.coverUrl} alt="" />
          <span className="search-card__play" aria-hidden>
            <Play size={20} weight="fill" />
          </span>
          <span className="search-card__duration">
            {formatDuration(track.durationMs / 1000)}
          </span>
          {track.cachedAt !== null && (
            <span className="search-card__badge">已缓存</span>
          )}
          {track.cachedAt === null && (
            <span className="search-card__badge">网易云</span>
          )}
        </span>
        <span className="search-card__title" title={track.title}>
          {track.title}
        </span>
        <span className="search-card__meta">
          <span className="search-card__author" title={track.artist}>
            {track.artist || track.albumName || "未知歌手"}
          </span>
          {subText && <span className="search-card__sub">{subText}</span>}
        </span>
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
