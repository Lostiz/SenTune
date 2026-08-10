import { useCallback, useEffect, useState } from "react";
import { MotionConfig, motion } from "motion/react";
import { AppShell } from "./components/layout/AppShell";
import { Toaster } from "./components/common/Toaster";
import type { PageId } from "./components/layout/Sidebar";
import { SearchPage } from "./pages/Search";
import { SettingsPage } from "./pages/Settings";
import { HomePage } from "./pages/Home";
import { FavoritesPage } from "./pages/Favorites";
import { HistoryPage } from "./pages/History";
import { PlaylistsPage } from "./pages/Playlists";
import { PlaylistPickerModal } from "./components/library/PlaylistPickerModal";
import { AboutModal } from "./components/common/AboutModal";
import { useLibraryStore } from "./stores/libraryStore";
import { useSettingsStore } from "./stores/settingsStore";

const PAGES: PageId[] = [
  "home",
  "search",
  "playlists",
  "favorites",
  "history",
  "settings",
];

const PAGE_TITLES: Record<PageId, string> = {
  home: "首页",
  search: "搜索",
  playlists: "歌单",
  favorites: "收藏",
  history: "历史",
  settings: "设置",
  about: "关于",
};

function pageFromHash(): PageId {
  const raw = window.location.hash.replace(/^#\/?/, "");
  return (PAGES as string[]).includes(raw) ? (raw as PageId) : "home";
}

function PlaceholderPage({ page }: { page: PageId }) {
  return (
    <section className="page">
      <h1>{PAGE_TITLES[page]}</h1>
      <p className="page__hint">SenTune 正在搭建中，此页面将在后续阶段实现。</p>
    </section>
  );
}

export default function App() {
  const [page, setPage] = useState<PageId>(pageFromHash);
  const theme = useSettingsStore((state) => state.theme);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const refreshAll = useLibraryStore((state) => state.refreshAll);

  useEffect(() => {
    void refreshAll();
  }, [refreshAll]);

  useEffect(() => {
    const onHashChange = () => setPage(pageFromHash());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  useEffect(() => {
    document.title = `SenTune · ${PAGE_TITLES[page]}`;
  }, [page]);

  const navigate = useCallback((next: PageId) => {
    window.location.hash = `/${next}`;
    setPage(next);
  }, []);

  return (
    <MotionConfig reducedMotion="user">
      <AppShell page={page} onNavigate={navigate}>
        <motion.div
          key={page}
          className="page-transition"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.18, ease: "easeOut" }}
        >
          {page === "home" && <HomePage />}
          {page === "settings" && <SettingsPage />}
          {page === "search" && <SearchPage />}
          {page === "favorites" && <FavoritesPage />}
          {page === "history" && <HistoryPage />}
          {page === "playlists" && <PlaylistsPage />}
          {page !== "home" &&
            page !== "settings" &&
            page !== "search" &&
            page !== "favorites" &&
            page !== "history" &&
            page !== "playlists" && <PlaceholderPage page={page} />}
        </motion.div>
      </AppShell>
      <PlaylistPickerModal />
      <AboutModal />
      <Toaster />
    </MotionConfig>
  );
}
