export interface VideoItem {
  bvid: string;
  title: string;
  pic: string;
  duration: number;
  author: string;
  play: number;
  danmaku: number;
}

export interface SearchPage {
  items: VideoItem[];
  page: number;
  pageSize: number;
  total: number;
  totalPages: number;
}

export interface VideoDetail {
  bvid: string;
  cid: number;
  title: string;
  cover: string;
  duration: number;
  author: string;
  play: number;
  pages: VideoPage[];
}

export interface VideoPage {
  cid: number;
  page: number;
  part: string;
  duration: number;
}

export interface TrackInfo {
  bvid: string;
  cid: number;
  title: string;
  coverUrl: string;
  author: string;
  duration: number;
  audioId: number;
  codec: string;
  cachePath: string | null;
  cachedAt: number | null;
  lastPlayedAt: number | null;
  playCount: number;
}

export interface LocalTrack {
  id: number;
  path: string;
  folderId: number | null;
  title: string;
  artist: string;
  album: string;
  duration: number;
  codec: string;
  size: number;
  modifiedAt: number;
  coverPath: string | null;
  addedAt: number;
  lastPlayedAt: number | null;
  playCount: number;
}

export interface LocalFolder {
  id: number;
  path: string;
  addedAt: number;
  trackCount: number;
}

export interface LocalScanResult {
  folderId: number;
  path: string;
  added: number;
  updated: number;
  removed: number;
  skipped: number;
}

export interface FavoriteItem {
  track: TrackInfo;
  createdAt: number;
}

export interface HistoryItem {
  track: TrackInfo;
  playedAt: number;
}

export interface PlaylistSummary {
  id: number;
  name: string;
  createdAt: number;
  trackCount: number;
}

export interface PlaylistDetail {
  id: number;
  name: string;
  tracks: TrackInfo[];
}

export interface CacheSettings {
  cachePath: string | null;
  keepDays: number;
  capacityLimitGb: number;
}

export interface CacheStatus {
  totalSize: number;
  fileCount: number;
  capacityLimitGb: number;
  cachePath: string;
}
