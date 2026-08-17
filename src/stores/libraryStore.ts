import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import type {
  FavoriteItem,
  HistoryItem,
  NeteaseFavoriteItem,
  NeteaseHistoryItem,
  NeteaseSong,
  PlaylistDetail,
  PlaylistSummary,
  TrackInfo,
} from "../types/models";

interface LibraryState {
  favorites: FavoriteItem[];
  neteaseFavorites: NeteaseFavoriteItem[];
  playlists: PlaylistSummary[];
  history: HistoryItem[];
  neteaseHistory: NeteaseHistoryItem[];
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
  addNeteaseFavorite: (song: NeteaseSong) => Promise<void>;
  removeNeteaseFavorite: (songId: number) => Promise<void>;
  isNeteaseFavorite: (songId: number) => boolean;
  clearHistory: () => Promise<void>;
  clearNeteaseHistory: () => Promise<void>;
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
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  favorites: [],
  neteaseFavorites: [],
  playlists: [],
  history: [],
  neteaseHistory: [],
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
      const [favorites, neteaseFavorites] = await Promise.all([
        invoke<FavoriteItem[]>("list_favorites"),
        invoke<NeteaseFavoriteItem[]>("list_netease_favorites"),
      ]);
      set({ favorites, neteaseFavorites });
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
      const [history, neteaseHistory] = await Promise.all([
        invoke<HistoryItem[]>("list_history"),
        invoke<NeteaseHistoryItem[]>("list_netease_history"),
      ]);
      set({ history, neteaseHistory });
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
  addNeteaseFavorite: async (song) => {
    await invoke("add_netease_favorite", { song });
    await get().refreshFavorites();
  },
  removeNeteaseFavorite: async (songId) => {
    await invoke("remove_netease_favorite", { songId });
    await get().refreshFavorites();
  },
  isNeteaseFavorite: (songId) =>
    get().neteaseFavorites.some((item) => item.track.songId === songId),
  clearHistory: async () => {
    await invoke("clear_history");
    await get().refreshHistory();
  },
  clearNeteaseHistory: async () => {
    await invoke("clear_netease_history");
    await get().refreshHistory();
  },
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
}));
