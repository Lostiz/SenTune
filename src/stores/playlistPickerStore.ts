import { create } from "zustand";
import type { TrackInfo } from "../types/models";

interface PlaylistPickerState {
  open: boolean;
  track: TrackInfo | null;
  openPicker: (track: TrackInfo) => void;
  closePicker: () => void;
}

export const usePlaylistPickerStore = create<PlaylistPickerState>((set) => ({
  open: false,
  track: null,
  openPicker: (track) => set({ open: true, track }),
  closePicker: () => set({ open: false, track: null }),
}));
