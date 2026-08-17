import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { pauseAudio, playAudio, seekAudio, setAudioVolume } from "../lib/audioController";
import { getProxyPort } from "../lib/proxy";
import { localTrackToQueueItem, queueItemToNeteaseSong } from "../lib/track";
import type {
  LocalTrack,
  NeteaseSong,
  NeteaseTrackInfo,
  VideoDetail,
  VideoItem,
} from "../types/models";

export interface QueueItem {
  source: "bili" | "local" | "netease";
  bvid: string;
  cid: number;
  path?: string;
  title: string;
  cover: string;
  author: string;
  duration: number;
  /** 网易云曲目完整数据（用于收藏/历史落库）。 */
  song?: NeteaseSong;
}

interface PlayerState {
  queue: QueueItem[];
  currentIndex: number | null;
  history: QueueItem[];
  isPlaying: boolean;
  currentTime: number;
  duration: number;
  streamUrl: string | null;
  streamId: string | null;
  cachePercent: number;
  loadingStream: boolean;
  playbackError: string | null;
  currentAudioId: number | null;
  volume: number;
  fullPlayerOpen: boolean;
  queuePanelOpen: boolean;
  setQueue: (items: QueueItem[], startIndex: number) => void;
  enqueue: (item: QueueItem) => void;
  playItem: (item: QueueItem) => void;
  playLocalTrack: (track: LocalTrack) => Promise<void>;
  playVideo: (item: VideoItem) => Promise<void>;
  playNeteaseSong: (song: NeteaseSong) => Promise<void>;
  playNeteaseTrack: (track: NeteaseTrackInfo) => Promise<void>;
  playAt: (index: number) => Promise<void>;
  togglePlay: () => void;
  setPlaying: (playing: boolean) => void;
  next: () => void;
  previous: () => void;
  seek: (time: number) => void;
  setCurrentTime: (time: number) => void;
  setDuration: (duration: number) => void;
  setPlaybackError: (message: string | null) => void;
  retryPlay: (excludeAudioId?: number) => Promise<void>;
  setVolume: (volume: number) => void;
  openFullPlayer: () => void;
  closeFullPlayer: () => void;
  toggleQueuePanel: () => void;
  _startStream: (excludeAudioId?: number) => Promise<void>;
}

export interface StreamStatus {
  streamId: string;
  bvid: string;
  cid: number;
  title: string;
  audioId: number;
  totalSize: number;
  downloaded: number;
  status: "downloading" | "completed" | "cancelled" | "failed";
  error: string | null;
  cachePath: string | null;
  cachePercent: number;
  port: number;
}

let pollTimer: number | null = null;
const retryCounts: Record<string, number> = {};

const MAX_HISTORY = 50;

function stopPolling() {
  if (pollTimer !== null) {
    window.clearInterval(pollTimer);
    pollTimer = null;
  }
}

function startPolling(streamId: string) {
  stopPolling();
  pollTimer = window.setInterval(async () => {
    try {
      const status = await invoke<StreamStatus>("get_stream_status", { streamId });
      const current = usePlayerStore.getState();
      if (current.streamId !== streamId) {
        stopPolling();
        return;
      }
      usePlayerStore.setState({ cachePercent: status.cachePercent });
      if (status.status === "completed") {
        stopPolling();
      } else if (status.status === "failed") {
        stopPolling();
        usePlayerStore.setState({
          playbackError: status.error ?? "音频流获取失败",
          loadingStream: false,
        });
      }
    } catch {
      stopPolling();
    }
  }, 1000);
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  queue: [],
  currentIndex: null,
  history: [],
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  streamUrl: null,
  streamId: null,
  cachePercent: 0,
  loadingStream: false,
  playbackError: null,
  currentAudioId: null,
  volume: (() => {
    try {
      const saved = Number(window.localStorage.getItem("sentune-volume"));
      if (Number.isFinite(saved)) return Math.min(1, Math.max(0, saved));
    } catch {
      // 忽略
    }
    return 1;
  })(),
  fullPlayerOpen: false,
  queuePanelOpen: false,
  setQueue: (items, startIndex) =>
    set({
      queue: items,
      currentIndex: startIndex,
      isPlaying: false,
      currentTime: 0,
      duration: 0,
      streamUrl: null,
      streamId: null,
      cachePercent: 0,
      playbackError: null,
    }),
  enqueue: (item) => {
    const { queue } = get();
    if (queue.length === 0) {
      set({ queue: [item], currentIndex: 0, isPlaying: false });
    } else {
      set({ queue: [...queue, item] });
    }
  },
  playItem: (item) => {
    const { queue, playAt } = get();
    const existing = queue.findIndex((entry) => entry.bvid === item.bvid);
    if (existing >= 0) {
      void playAt(existing);
    } else {
      const index = queue.length;
      get().enqueue(item);
      void playAt(index);
    }
  },
  playLocalTrack: async (track) => {
    const port = await getProxyPort();
    const item = localTrackToQueueItem(track, port);
    const { queue } = get();
    const existing = queue.findIndex((entry) => entry.bvid === item.bvid);
    if (existing >= 0) {
      await get().playAt(existing);
    } else {
      const index = queue.length;
      set({ queue: [...queue, item] });
      await get().playAt(index);
    }
  },
  playNeteaseSong: async (song) => {
    const item: QueueItem = {
      source: "netease",
      bvid: `netease:${song.id}`,
      cid: song.id,
      title: song.name,
      cover: song.picUrl,
      author: song.artist || song.albumName || "网易云音乐",
      duration: Math.max(1, Math.round(song.durationMs / 1000)),
      song,
    };
    const { queue } = get();
    const existing = queue.findIndex((entry) => entry.bvid === item.bvid);
    if (existing >= 0) {
      await get().playAt(existing);
    } else {
      const index = queue.length;
      set({ queue: [...queue, item] });
      await get().playAt(index);
    }
  },
  playNeteaseTrack: async (track) => {
    const song: NeteaseSong = {
      id: track.songId,
      name: track.title,
      artist: track.artist,
      albumName: track.albumName,
      picUrl: track.coverUrl,
      durationMs: track.durationMs,
      fee: track.fee,
    };
    await get().playNeteaseSong(song);
  },
  playVideo: async (item) => {
    let detail: VideoDetail;
    try {
      detail = await invoke<VideoDetail>("get_video_detail", {
        bvid: item.bvid,
      });
    } catch (error) {
      set({ playbackError: String(error) });
      return;
    }
    const items: QueueItem[] =
      detail.pages.length > 1
        ? detail.pages.map((page) => ({
            source: "bili",
            bvid: detail.bvid,
            cid: page.cid,
            title: `${detail.title} · ${page.part || `第 ${page.page} 集`}`,
            cover: detail.cover,
            author: detail.author,
            duration: page.duration,
          }))
        : [
            {
              source: "bili",
              bvid: detail.bvid,
              cid: detail.cid,
              title: detail.title,
              cover: detail.cover,
              author: detail.author,
              duration: detail.duration,
            },
          ];
    const { queue } = get();
    const startIndex = queue.length;
    set({ queue: [...queue, ...items] });
    await get().playAt(startIndex);
  },
  playAt: async (index) => {
    const { queue, streamId, history } = get();
    const item = queue[index];
    if (!item) return;
    set({
      history: [
        item,
        ...history.filter((entry) => entry.bvid !== item.bvid),
      ].slice(0, MAX_HISTORY),
    });
    if (streamId) {
      void invoke("cancel_stream", { streamId }).catch(() => undefined);
    }
    stopPolling();
    retryCounts[item.bvid] = 0;
    set({
      currentIndex: index,
      isPlaying: false,
      currentTime: 0,
      duration: 0,
      streamUrl: null,
      streamId: null,
      cachePercent: 0,
      loadingStream: true,
      playbackError: null,
      currentAudioId: null,
    });
    await get()._startStream(undefined);
  },
  togglePlay: () => {
    const { currentIndex, streamUrl, loadingStream, isPlaying } = get();
    if (currentIndex === null) return;
    if (!streamUrl && !loadingStream) {
      void get().playAt(currentIndex);
      return;
    }
    if (loadingStream) return;
    if (isPlaying) {
      pauseAudio();
      set({ isPlaying: false });
    } else {
      playAudio()?.catch((error: unknown) => {
        if ((error as DOMException | null)?.name !== "AbortError") {
          console.error("播放失败", error);
          set({ isPlaying: false });
        }
      });
      set({ isPlaying: true });
    }
  },
  setPlaying: (playing) => set({ isPlaying: playing }),
  next: () => {
    const { queue, currentIndex } = get();
    if (currentIndex === null || queue.length === 0) return;
    void get().playAt((currentIndex + 1) % queue.length);
  },
  previous: () => {
    const { queue, currentIndex } = get();
    if (currentIndex === null || queue.length === 0) return;
    void get().playAt((currentIndex - 1 + queue.length) % queue.length);
  },
  seek: (time) => {
    seekAudio(time);
    set({ currentTime: time });
  },
  setCurrentTime: (currentTime) => set({ currentTime }),
  setDuration: (duration) => set({ duration }),
  setPlaybackError: (playbackError) => set({ playbackError }),
  setVolume: (volume) => {
    try {
      window.localStorage.setItem("sentune-volume", String(volume));
    } catch {
      // 忽略
    }
    setAudioVolume(volume);
    set({ volume });
  },
  openFullPlayer: () => set({ fullPlayerOpen: true, queuePanelOpen: false }),
  closeFullPlayer: () => set({ fullPlayerOpen: false }),
  toggleQueuePanel: () =>
    set((state) => ({ queuePanelOpen: !state.queuePanelOpen })),
  retryPlay: async (excludeAudioId) => {
    const { queue, currentIndex } = get();
    const item = currentIndex === null ? null : queue[currentIndex];
    if (!item) return;
    if (item.source === "local") return;
    const count = (retryCounts[item.bvid] ?? 0) + 1;
    if (count > 2) return;
    retryCounts[item.bvid] = count;
    set({ loadingStream: true, playbackError: null });
    await get()._startStream(excludeAudioId);
  },
  _startStream: async (excludeAudioId) => {
    const { queue, currentIndex } = get();
    const item = currentIndex === null ? null : queue[currentIndex];
    if (!item) return;
    if (item.source === "local") {
      const port = await getProxyPort();
      const url = `http://127.0.0.1:${port}/local?path=${encodeURIComponent(
        item.path ?? "",
      )}`;
      if (get().currentIndex !== currentIndex) return;
      set({
        streamUrl: url,
        streamId: null,
        cachePercent: 0,
        loadingStream: false,
        playbackError: null,
        currentAudioId: null,
      });
      void invoke("add_local_history", { id: item.cid }).catch(() => undefined);
      return;
    }
    if (item.source === "netease") {
      // 走本地流代理：已缓存直接本地播放，否则边下边播（128K）。
      try {
        const status = await invoke<StreamStatus>("start_netease_stream", {
          song: item.song ?? queueItemToNeteaseSong(item),
        });
        if (get().currentIndex !== currentIndex) return;
        const url = `http://127.0.0.1:${status.port}/stream/${status.streamId}`;
        set({
          streamUrl: url,
          streamId: status.streamId,
          cachePercent: status.cachePercent,
          loadingStream: false,
          playbackError: null,
          currentAudioId: null,
        });
        if (item.song) {
          void invoke("add_netease_history", { song: item.song }).catch(
            () => undefined,
          );
        }
        startPolling(status.streamId);
      } catch (error) {
        set({ loadingStream: false, playbackError: String(error) });
      }
      return;
    }
    try {
      const status = await invoke<StreamStatus>("start_stream", {
        bvid: item.bvid,
        cid: item.cid ?? null,
        excludeAudioId: excludeAudioId ?? null,
      });
      if (get().currentIndex !== currentIndex) return;
      const url = `http://127.0.0.1:${status.port}/stream/${status.streamId}`;
      set({
        streamUrl: url,
        streamId: status.streamId,
        cachePercent: status.cachePercent,
        loadingStream: false,
        playbackError: null,
        currentAudioId: status.audioId,
      });
      void invoke("add_history", { bvid: item.bvid, cid: item.cid ?? null }).catch(
        () => undefined,
      );
      startPolling(status.streamId);
    } catch (error) {
      const message = String(error);
      set({ loadingStream: false, playbackError: message });
    }
  },
}));
