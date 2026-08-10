import type { QueueItem } from "../stores/playerStore";
import type { TrackInfo } from "../types/models";

export function trackToQueueItem(track: TrackInfo): QueueItem {
  return {
    bvid: track.bvid,
    cid: track.cid,
    title: track.title,
    cover: track.coverUrl,
    author: track.author,
    duration: track.duration,
  };
}

export function queueItemToTrackInfo(item: QueueItem): TrackInfo {
  return {
    bvid: item.bvid,
    cid: item.cid,
    title: item.title,
    coverUrl: item.cover,
    author: item.author,
    duration: item.duration,
    audioId: 0,
    codec: "",
    cachePath: null,
    cachedAt: null,
    lastPlayedAt: null,
    playCount: 0,
  };
}

export function formatDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function formatDateTime(timestamp: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1000));
}
