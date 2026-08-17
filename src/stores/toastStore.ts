import { create } from "zustand";

interface ToastState {
  message: string | null;
  showToast: (message: string) => void;
  hideToast: () => void;
}

let hideTimer: number | null = null;

export const useToastStore = create<ToastState>((set) => ({
  message: null,
  showToast: (message) => {
    if (hideTimer !== null) {
      window.clearTimeout(hideTimer);
    }
    set({ message });
    hideTimer = window.setTimeout(() => {
      set({ message: null });
      hideTimer = null;
    }, 2600);
  },
  hideToast: () => {
    if (hideTimer !== null) {
      window.clearTimeout(hideTimer);
      hideTimer = null;
    }
    set({ message: null });
  },
}));
