import { useCallback, useEffect, useState } from "react";
import {
  ArrowsClockwise,
  FolderPlus,
  Heart,
  MagnifyingGlass,
  MusicNotes,
  Play,
  Trash,
} from "@phosphor-icons/react";
import { invoke } from "@tauri-apps/api/core";
import { EmptyState } from "../components/common/EmptyState";
import { IconButton } from "../components/common/IconButton";
import { getProxyPort } from "../lib/proxy";
import { formatDuration } from "../lib/track";
import { usePlayerStore } from "../stores/playerStore";
import { useToastStore } from "../stores/toastStore";
import type {
  LocalFolder,
  LocalScanResult,
  LocalTrack,
} from "../types/models";

function LocalCover({ track }: { track: LocalTrack }) {
  const [url, setUrl] = useState("");
  useEffect(() => {
    let cancelled = false;
    void getProxyPort()
      .then((port) => {
        if (!cancelled) {
          setUrl(
            track.coverPath
              ? `http://127.0.0.1:${port}/local-cover/${track.id}`
              : "",
          );
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [track.id, track.coverPath]);
  if (!url) {
    return (
      <span className="local-row__cover cover-fallback" aria-hidden>
        <MusicNotes size={20} weight="light" />
      </span>
    );
  }
  return (
    <img
      className="local-row__cover"
      src={url}
      alt=""
      width={48}
      height={48}
      onError={(event) => {
        event.currentTarget.style.display = "none";
      }}
    />
  );
}

export function LocalPage() {
  const [tab, setTab] = useState<"all" | "favorites" | "history">("all");
  const [folders, setFolders] = useState<LocalFolder[]>([]);
  const [tracks, setTracks] = useState<LocalTrack[]>([]);
  const [favorites, setFavorites] = useState<Set<number>>(new Set());
  const [query, setQuery] = useState("");
  const [selectedFolder, setSelectedFolder] = useState<number | null>(null);
  const [importing, setImporting] = useState(false);
  const [scanning, setScanning] = useState<number | null>(null);
  const showToast = useToastStore((state) => state.showToast);
  const playLocalTrack = usePlayerStore((state) => state.playLocalTrack);

  const refresh = useCallback(async () => {
    const folderList = await invoke<LocalFolder[]>("list_local_folders");
    const trackList =
      tab === "favorites"
        ? await invoke<LocalTrack[]>("list_local_favorites")
        : tab === "history"
          ? await invoke<LocalTrack[]>("list_local_history")
          : await invoke<LocalTrack[]>("list_local_tracks", {
              query: query.trim() || null,
              folderId: selectedFolder,
            });
    const favoriteList = await invoke<LocalTrack[]>("list_local_favorites");
    setFolders(folderList);
    setTracks(trackList);
    setFavorites(new Set(favoriteList.map((track) => track.id)));
  }, [query, selectedFolder, tab]);

  useEffect(() => {
    void refresh().catch((error: unknown) => showToast(String(error)));
  }, [refresh, showToast]);

  const importFolder = async () => {
    try {
      const path = await invoke<string | null>("pick_local_folder");
      if (!path) return;
      setImporting(true);
      await invoke("add_local_folder", { path });
      showToast("已导入本地音乐");
      await refresh();
    } catch (error) {
      showToast(String(error));
    } finally {
      setImporting(false);
    }
  };

  const rescan = async (id: number) => {
    setScanning(id);
    try {
      const result = await invoke<LocalScanResult>("rescan_local_folder", { id });
      showToast(
        `扫描完成：新增 ${result.added}，更新 ${result.updated}，移除 ${result.removed}`,
      );
      await refresh();
    } catch (error) {
      showToast(String(error));
    } finally {
      setScanning(null);
    }
  };

  const removeFolder = async (id: number) => {
    try {
      await invoke("remove_local_folder", { id });
      if (selectedFolder === id) setSelectedFolder(null);
      await refresh();
    } catch (error) {
      showToast(String(error));
    }
  };

  const removeTrack = async (id: number) => {
    try {
      await invoke("remove_local_track", { id });
      await refresh();
    } catch (error) {
      showToast(String(error));
    }
  };

  const toggleFavorite = async (track: LocalTrack) => {
    const has = favorites.has(track.id);
    try {
      await invoke(has ? "remove_local_favorite" : "add_local_favorite", {
        id: track.id,
      });
      setFavorites((prev) => {
        const next = new Set(prev);
        if (has) {
          next.delete(track.id);
        } else {
          next.add(track.id);
        }
        return next;
      });
    } catch (error) {
      showToast(String(error));
    }
  };

  return (
    <section className="page page--library">
      <p className="page__subtitle">资料库</p>
      <div className="local-page__header">
        <h1>本地音乐</h1>
        <button
          type="button"
          className="button button--primary"
          onClick={() => void importFolder()}
          disabled={importing}
        >
          <FolderPlus size={16} aria-hidden />
          {importing ? "导入中…" : "导入文件夹"}
        </button>
      </div>

      {folders.length > 0 && (
        <div className="local-page__tabs" role="tablist" aria-label="本地音乐视图">
          {(
            [
              ["all", "全部"],
              ["favorites", "收藏"],
              ["history", "最近播放"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              type="button"
              role="tab"
              aria-selected={tab === value}
              className={`local-page__tab${
                tab === value ? " local-page__tab--active" : ""
              }`}
              onClick={() => setTab(value)}
            >
              {label}
            </button>
          ))}
        </div>
      )}

      {folders.length > 0 && (
        <div className="local-page__folders">
          {folders.map((folder) => (
            <div key={folder.id} className="local-folder">
              <button
                type="button"
                className={`local-folder__select${
                  selectedFolder === folder.id ? " local-folder__select--active" : ""
                }`}
                onClick={() =>
                  setSelectedFolder(selectedFolder === folder.id ? null : folder.id)
                }
                title={folder.path}
              >
                <MusicNotes size={14} aria-hidden />
                <span className="local-folder__name">
                  {folder.path.split(/[\\/]/).pop()}
                </span>
                <span className="local-folder__count">{folder.trackCount} 首</span>
              </button>
              <IconButton
                icon={ArrowsClockwise}
                label={`重新扫描 ${folder.path}`}
                iconSize={14}
                disabled={scanning === folder.id}
                onClick={() => void rescan(folder.id)}
              />
              <IconButton
                icon={Trash}
                label={`移除文件夹 ${folder.path}`}
                iconSize={14}
                onClick={() => void removeFolder(folder.id)}
              />
            </div>
          ))}
        </div>
      )}

      {folders.length > 0 && (
        <div className="local-page__search">
          <MagnifyingGlass size={15} aria-hidden />
          <label htmlFor="local-search" className="visually-hidden">
            搜索本地音乐
          </label>
          <input
            id="local-search"
            type="search"
            placeholder="搜索标题 / 艺术家 / 专辑"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
      )}

      {folders.length === 0 ? (
        <EmptyState
          icon={MusicNotes}
          title="还没有本地音乐"
          hint="导入一个音乐文件夹，自动扫描并解析音频元数据"
          action={
            <button
              type="button"
              className="button button--primary"
              onClick={() => void importFolder()}
              disabled={importing}
            >
              <FolderPlus size={16} aria-hidden />
              导入文件夹
            </button>
          }
        />
      ) : tracks.length === 0 ? (
        <EmptyState
          icon={MusicNotes}
          title="没有匹配的曲目"
          hint="换一个关键词，或导入其他文件夹"
        />
      ) : (
        <div className="local-page__list">
          {tracks.map((track) => {
            const fav = favorites.has(track.id);
            return (
              <div key={track.id} className="search-row-wrap">
                <div
                  className="search-row local-row"
                  role="button"
                  tabIndex={0}
                  onClick={() => void playLocalTrack(track)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      void playLocalTrack(track);
                    }
                  }}
                >
                  <LocalCover track={track} />
                  <div className="search-row__info">
                    <p className="search-row__title" title={track.title}>
                      {track.title}
                    </p>
                    <p className="search-row__meta">
                      {[track.artist, track.album].filter(Boolean).join(" · ") ||
                        "未知艺术家"}
                      {" · "}
                      {formatDuration(track.duration)}
                    </p>
                  </div>
                  <div className="search-row__more">
                    <IconButton
                      icon={Play}
                      label={`播放 ${track.title}`}
                      iconSize={16}
                      onClick={(event) => {
                        event.stopPropagation();
                        void playLocalTrack(track);
                      }}
                    />
                    <IconButton
                      icon={Heart}
                      weight={fav ? "fill" : "regular"}
                      label={fav ? "取消收藏" : "收藏"}
                      iconSize={16}
                      className={fav ? "local-row__fav--active" : ""}
                      onClick={(event) => {
                        event.stopPropagation();
                        void toggleFavorite(track);
                      }}
                    />
                    <IconButton
                      icon={Trash}
                      label={`移除 ${track.title}`}
                      iconSize={16}
                      onClick={(event) => {
                        event.stopPropagation();
                        void removeTrack(track.id);
                      }}
                    />
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
