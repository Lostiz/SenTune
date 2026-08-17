import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import type {
  NeteaseArtist,
  NeteaseArtistSearchPage,
  NeteaseSearchPage,
  NeteaseSong,
  SearchPage,
  VideoItem,
} from "../types/models";
import { useSourceStore, type MusicSource } from "./sourceStore";

interface SearchState {
  keyword: string;
  source: MusicSource;
  items: VideoItem[];
  neteaseItems: NeteaseSong[];
  neteaseArtists: NeteaseArtist[];
  page: number;
  total: number;
  totalPages: number;
  loading: boolean;
  error: string | null;
  search: (keyword: string, page?: number) => Promise<void>;
  goToPage: (page: number) => void;
  clear: () => void;
}

let requestToken = 0;

export const useSearchStore = create<SearchState>((set, get) => ({
  keyword: "",
  source: "netease",
  items: [],
  neteaseItems: [],
  neteaseArtists: [],
  page: 1,
  total: 0,
  totalPages: 0,
  loading: false,
  error: null,
  search: async (keyword, page = 1) => {
    const token = ++requestToken;
    const trimmed = keyword.trim();
    const source = useSourceStore.getState().source;
    if (!trimmed) {
      set({
        keyword: "",
        source,
        items: [],
        neteaseItems: [],
          neteaseArtists: [],

        page: 1,
        total: 0,
        totalPages: 0,
        loading: false,
        error: null,
      });
      return;
    }
    set({
      keyword: trimmed,
      source,
      items: [],
      neteaseItems: [],
        neteaseArtists: [],

      page,
      total: 0,
      totalPages: 0,
      loading: true,
      error: null,
    });
    try {
      if (source === "netease") {
        const result = await invoke<NeteaseSearchPage>("search_netease", {
          keyword: trimmed,
          page,
        });
        if (token !== requestToken) return;
        set({
          neteaseItems: result.items,
          page: result.page,
          total: result.total,
          totalPages: result.totalPages,
          loading: false,
        });
          try {
            const artistResult = await invoke<NeteaseArtistSearchPage>(
              "search_netease_artists",
              { keyword: trimmed, page: 1 },
            );
            if (token !== requestToken) return;
            set({ neteaseArtists: artistResult.items });
          } catch {
            if (token !== requestToken) return;
            set({ neteaseArtists: [] });
          }

        return;
      }
      const result = await invoke<SearchPage>("search_videos", {
        keyword: trimmed,
        page,
      });
      if (token !== requestToken) return;
      set({
        items: result.items,
        page: result.page,
        total: result.total,
        totalPages: result.totalPages,
        loading: false,
      });
    } catch (error) {
      if (token !== requestToken) return;
      set({ loading: false, error: String(error) });
    }
  },
  goToPage: (page) => {
    const { keyword, page: currentPage, loading } = get();
    if (!keyword || loading || page === currentPage || page < 1) return;
    void get().search(keyword, page);
  },
  clear: () => {
    requestToken += 1;
    set({
      keyword: "",
      source: useSourceStore.getState().source,
      items: [],
      neteaseItems: [],
        neteaseArtists: [],

      page: 1,
      total: 0,
      totalPages: 0,
      loading: false,
      error: null,
    });
  },
}));
