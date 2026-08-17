import { create } from "zustand";

export type MusicSource = "bilibili" | "netease";

interface SourceState {
  source: MusicSource;
  setSource: (source: MusicSource) => void;
}

export const useSourceStore = create<SourceState>((set) => ({
  source: "netease",
  setSource: (source) => set({ source }),
}));
