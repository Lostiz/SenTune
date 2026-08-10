import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { CacheSettings, CacheStatus } from "../types/models";

export type Theme = "dark" | "light";

export interface SettingsState {
  cache: CacheSettings;
  cacheStatus: CacheStatus | null;
  theme: Theme;
  setTheme: (theme: Theme) => void;
  loadCacheSettings: () => Promise<void>;
  saveCacheSettings: (
    patch: Partial<Omit<CacheSettings, "cachePath">> & { cachePath?: string | null },
  ) => Promise<void>;
  refreshCacheStatus: () => Promise<void>;
  clearCache: (olderThanDays?: number) => Promise<CacheStatus | null>;
  pickCacheDir: () => Promise<string | null>;
}

const STORAGE_KEY = "sentune-theme";

function initialTheme(): Theme {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark") return saved;
  } catch {
    // 忽略 localStorage 不可用
  }
  return "dark";
}

export const useSettingsStore = create<SettingsState>((set) => ({
  cache: {
    cachePath: null,
    keepDays: 7,
    capacityLimitGb: 0,
  },
  cacheStatus: null,
  theme: initialTheme(),
  setTheme: (theme) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      // 忽略写入失败
    }
    document.documentElement.dataset.theme = theme;
    set({ theme });
  },
  loadCacheSettings: async () => {
    try {
      const cache = await invoke<CacheSettings>("get_cache_settings");
      set({ cache });
    } catch (error) {
      console.error("加载缓存设置失败", error);
    }
  },
  saveCacheSettings: async (patch) => {
    const next = await invoke<CacheSettings>("set_cache_settings", {
      keepDays: patch.keepDays ?? null,
      capacityLimitGb: patch.capacityLimitGb ?? null,
      cachePath: patch.cachePath ?? null,
    });
    set({ cache: next });
  },
  refreshCacheStatus: async () => {
    try {
      const cacheStatus = await invoke<CacheStatus>("get_cache_status");
      set({ cacheStatus });
    } catch (error) {
      console.error("读取缓存状态失败", error);
    }
  },
  clearCache: async (olderThanDays) => {
    await invoke("clear_cache", { olderThanDays: olderThanDays ?? null });
    const cacheStatus = await invoke<CacheStatus>("get_cache_status");
    set({ cacheStatus });
    return cacheStatus;
  },
  pickCacheDir: async () => {
    return await invoke<string | null>("pick_cache_dir");
  },
}));
