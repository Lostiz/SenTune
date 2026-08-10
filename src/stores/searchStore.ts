import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { SearchPage, VideoItem } from "../types/models";

interface SearchState {
  keyword: string;
  items: VideoItem[];
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
  items: [],
  page: 1,
  total: 0,
  totalPages: 0,
  loading: false,
  error: null,
  search: async (keyword, page = 1) => {
    const token = ++requestToken;
    const trimmed = keyword.trim();
    if (!trimmed) {
      set({
        keyword: "",
        items: [],
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
      items: [],
      page,
      total: 0,
      totalPages: 0,
      loading: true,
      error: null,
    });
    try {
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
      items: [],
      page: 1,
      total: 0,
      totalPages: 0,
      loading: false,
      error: null,
    });
  },
}));
