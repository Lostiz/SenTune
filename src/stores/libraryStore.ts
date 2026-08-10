import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import type {
  FavoriteItem,
  HistoryItem,
  PlaylistDetail,
  PlaylistSummary,
  TrackInfo,
} from "../types/models";

interface LibraryState {
  favorites: FavoriteItem[];
  playlists: PlaylistSummary[];
  history: HistoryItem[];
  cachedTracks: TrackInfo[];
  loaded: boolean;
  refreshAll: () => Promise<void>;
  refreshFavorites: () => Promise<void>;
  refreshPlaylists: () => Promise<void>;
  refreshHistory: () => Promise<void>;
  refreshCached: () => Promise<void>;
  addFavorite: (track: TrackInfo) => Promise<void>;
  removeFavorite: (bvid: string, cid: number) => Promise<void>;
  isFavorite: (bvid: string, cid: number) => boolean;
  openPlaylist: (id: number) => Promise<PlaylistDetail | null>;
  createPlaylist: (name: string) => Promise<number | null>;
  renamePlaylist: (id: number, name: string) => Promise<void>;
  deletePlaylist: (id: number) => Promise<void>;
  addToPlaylist: (playlistId: number, track: TrackInfo) => Promise<void>;
  removeFromPlaylist: (
    playlistId: number,
    bvid: string,
    cid: number,
  ) => Promise<void>;
  moveInPlaylist: (
    playlistId: number,
    bvid: string,
    cid: number,
    toPosition: number,
  ) => Promise<void>;
  clearHistory: () => Promise<void>;
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  favorites: [],
  playlists: [],
  history: [],
  cachedTracks: [],
  loaded: false,
  refreshAll: async () => {
    await Promise.all([
      get().refreshFavorites(),
      get().refreshPlaylists(),
      get().refreshHistory(),
      get().refreshCached(),
    ]);
    set({ loaded: true });
  },
  refreshFavorites: async () => {
    try {
      const favorites = await invoke<FavoriteItem[]>("list_favorites");
      set({ favorites });
    } catch (error) {
      console.error("加载收藏失败", error);
    }
  },
  refreshPlaylists: async () => {
    try {
      const playlists = await invoke<PlaylistSummary[]>("list_playlists");
      set({ playlists });
    } catch (error) {
      console.error("加载歌单失败", error);
    }
  },
  refreshHistory: async () => {
    try {
      const history = await invoke<HistoryItem[]>("list_history");
      set({ history });
    } catch (error) {
      console.error("加载历史失败", error);
    }
  },
  refreshCached: async () => {
    try {
      const cachedTracks = await invoke<TrackInfo[]>("list_cached_tracks");
      set({ cachedTracks });
    } catch (error) {
      console.error("加载缓存曲目失败", error);
    }
  },
  addFavorite: async (track) => {
    await invoke("add_favorite", { bvid: track.bvid, cid: track.cid });
    await get().refreshFavorites();
  },
  removeFavorite: async (bvid, cid) => {
    await invoke("remove_favorite", { bvid, cid });
    await get().refreshFavorites();
  },
  isFavorite: (bvid, cid) =>
    get().favorites.some(
      (item) => item.track.bvid === bvid && item.track.cid === cid,
    ),
  openPlaylist: async (id) => {
    try {
      return await invoke<PlaylistDetail>("get_playlist", { id });
    } catch (error) {
      console.error("加载歌单失败", error);
      return null;
    }
  },
  createPlaylist: async (name) => {
    try {
      const id = await invoke<number>("create_playlist", { name });
      await get().refreshPlaylists();
      return id;
    } catch (error) {
      console.error("创建歌单失败", error);
      return null;
    }
  },
  renamePlaylist: async (id, name) => {
    await invoke("rename_playlist", { id, name });
    await get().refreshPlaylists();
  },
  deletePlaylist: async (id) => {
    await invoke("delete_playlist", { id });
    await get().refreshPlaylists();
  },
  addToPlaylist: async (playlistId, track) => {
    await invoke("add_to_playlist", {
      playlistId,
      bvid: track.bvid,
      cid: track.cid,
    });
    await get().refreshPlaylists();
  },
  removeFromPlaylist: async (playlistId, bvid, cid) => {
    await invoke("remove_from_playlist", { playlistId, bvid, cid });
    await get().refreshPlaylists();
  },
  moveInPlaylist: async (playlistId, bvid, cid, toPosition) => {
    await invoke("move_in_playlist", {
      playlistId,
      bvid,
      cid,
      toPosition,
    });
    await get().refreshPlaylists();
  },
  clearHistory: async () => {
    await invoke("clear_history");
    await get().refreshHistory();
  },
}));
