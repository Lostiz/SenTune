import { create } from "zustand";

interface AboutState {
  open: boolean;
  openAbout: () => void;
  closeAbout: () => void;
}

export const useAboutStore = create<AboutState>((set) => ({
  open: false,
  openAbout: () => set({ open: true }),
  closeAbout: () => set({ open: false }),
}));
