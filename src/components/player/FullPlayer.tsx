import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  Heart,
  ListPlus,
  Minus,
  MusicNotes,
  Pause,
  Play,
  Queue,
  SkipBack,
  SkipForward,
  SpeakerHigh,
  SpeakerLow,
  Square,
  X,
} from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useLibraryStore } from "../../stores/libraryStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { usePlayerStore } from "../../stores/playerStore";
import { useToastStore } from "../../stores/toastStore";
import { IconButton } from "../common/IconButton";
import { ProgressBar } from "./ProgressBar";
import { queueItemToNeteaseSong, queueItemToTrackInfo } from "../../lib/track";
import { findActiveLyricIndex, mergeLyrics, type LyricLine } from "../../lib/lyrics";
import { useResolvedCover } from "../common/CoverImage";
import type { NeteaseLyric } from "../../types/models";

export function FullPlayer() {
  const open = usePlayerStore((state) => state.fullPlayerOpen);
  const item = usePlayerStore((state) =>
    state.currentIndex === null ? null : state.queue[state.currentIndex],
  );
  const isPlaying = usePlayerStore((state) => state.isPlaying);
  const currentTime = usePlayerStore((state) => state.currentTime);
  const duration = usePlayerStore((state) => state.duration);
  const volume = usePlayerStore((state) => state.volume);
  const loadingStream = usePlayerStore((state) => state.loadingStream);
  const cachePercent = usePlayerStore((state) => state.cachePercent);
  const streamId = usePlayerStore((state) => state.streamId);
  const closeFullPlayer = usePlayerStore((state) => state.closeFullPlayer);
  const togglePlay = usePlayerStore((state) => state.togglePlay);
  const previous = usePlayerStore((state) => state.previous);
  const next = usePlayerStore((state) => state.next);
  const seek = usePlayerStore((state) => state.seek);
  const setVolume = usePlayerStore((state) => state.setVolume);
  const toggleQueuePanel = usePlayerStore((state) => state.toggleQueuePanel);
  const isFavorite = useLibraryStore((state) => state.isFavorite);
  const addFavorite = useLibraryStore((state) => state.addFavorite);
  const removeFavorite = useLibraryStore((state) => state.removeFavorite);
  const isNeteaseFavorite = useLibraryStore((state) => state.isNeteaseFavorite);
  const addNeteaseFavorite = useLibraryStore((state) => state.addNeteaseFavorite);
  const removeNeteaseFavorite = useLibraryStore(
    (state) => state.removeNeteaseFavorite,
  );
  const openPicker = usePlaylistPickerStore((state) => state.openPicker);
  const showToast = useToastStore((state) => state.showToast);
    const appWindow = getCurrentWindow();

  const {
    url: remoteCover,
    failed: coverFailed,
    setFailed: setCoverFailed,
  } = useResolvedCover(item?.cover);
  const [localCover, setLocalCover] = useState<string | null>(null);
    const [lyricData, setLyricData] = useState<NeteaseLyric | null>(null);
    const [lyricLoading, setLyricLoading] = useState(false);
    const [lyricError, setLyricError] = useState<string | null>(null);
    const activeLyricRef = useRef<HTMLDivElement | null>(null);
      const lyricsViewportRef = useRef<HTMLElement | null>(null);
      const lyricsTrackRef = useRef<HTMLDivElement | null>(null);


  useEffect(() => {
    setLocalCover(null);
    // 仅 B 站曲目查询本地封面缓存；本地/网易云曲目直接使用原封面地址。
    if (!item || item.source !== "bili") return;
    let cancelled = false;
    void invoke<string>("get_cover_url", { bvid: item.bvid })
      .then((url) => {
        if (!cancelled && url) setLocalCover(url);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [item?.bvid]);

    useEffect(() => {
      setLyricData(null);
      setLyricError(null);
      if (!item || item.source !== "netease") {
        setLyricLoading(false);
        return;
      }

      let cancelled = false;
      setLyricLoading(true);
      invoke<NeteaseLyric>("netease_lyric", { songId: item.cid })
        .then((data) => {
          if (!cancelled) setLyricData(data);
        })
        .catch((error: unknown) => {
          if (!cancelled) setLyricError(String(error));
        })
        .finally(() => {
          if (!cancelled) setLyricLoading(false);
        });

      return () => {
        cancelled = true;
      };
    }, [item?.bvid, item?.source, item?.cid]);


  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeFullPlayer();
      if (event.key === " ") {
        event.preventDefault();
        togglePlay();
      }
      if (event.key === "ArrowRight") {
        event.preventDefault();
        seek(Math.min(duration, currentTime + 5));
      }
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        seek(Math.max(0, currentTime - 5));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, closeFullPlayer, togglePlay, seek, duration, currentTime]);

  const cover = localCover ?? remoteCover ?? item?.cover;
  const fav =
    item?.source === "bili"
      ? isFavorite(item.bvid, item.cid)
      : item?.source === "netease"
        ? isNeteaseFavorite(item.cid)
        : false;

    const lyricLines = useMemo<LyricLine[]>(
      () => mergeLyrics(lyricData?.lyric, lyricData?.translatedLyric),
      [lyricData],
    );
    const activeLyricIndex = useMemo(
      () => findActiveLyricIndex(lyricLines, currentTime),
      [lyricLines, currentTime],
    );

    useEffect(() => {
      // Use the layout effect below for centered Netease-style scrolling.
    }, [activeLyricIndex]);

      useLayoutEffect(() => {
        const viewport = lyricsViewportRef.current;
        const track = lyricsTrackRef.current;
        const activeLine = activeLyricRef.current;

        if (!viewport || !track || !activeLine || activeLyricIndex < 0) return;

        const offset =
          activeLine.offsetTop -
          viewport.clientHeight / 2 +
          activeLine.clientHeight / 2;
        track.style.transform = `translateY(${-offset}px)`;
      }, [activeLyricIndex, lyricLines]);



  return (
    <AnimatePresence>
      {open && item && (
        <motion.div
          className="full-player"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.25 }}
          role="dialog"
          aria-modal="true"
          aria-label="全屏播放器"
        >
          <div
            className="full-player__backdrop"
            style={{ backgroundImage: `url("${cover}")` }}
            aria-hidden
          />
          <div className="full-player__scrim" aria-hidden />

          <div className="full-player__top" data-tauri-drag-region>
            <IconButton
              icon={X}
              label="收起全屏播放器"
              onClick={closeFullPlayer}
            />
            <span className="full-player__title" translate="no" data-tauri-drag-region>
              SenTune
            </span>
            <IconButton
              icon={Queue}
              label="打开播放队列"
              onClick={toggleQueuePanel}
            />
              <div className="full-player__window-controls">
                <IconButton
                  icon={Minus}
                  label="最小化"
                  onClick={() => void appWindow.minimize()}
                />
                <IconButton
                  icon={Square}
                  label="最大化/还原"
                  onClick={() => void appWindow.toggleMaximize()}
                />
                <IconButton
                  icon={X}
                  label="关闭窗口"
                  onClick={() => void appWindow.close()}
                />
              </div>

          </div>

          <div
              className={`full-player__body${
                item.source === "netease" ? " full-player__body--with-lyrics" : ""
              }`}
            >
              <div className="full-player__main">
            {coverFailed && !localCover ? (
              <motion.div
                className="full-player__cover cover-fallback"
                layoutId="now-playing-cover"
                aria-hidden
              >
                <MusicNotes size={64} weight="light" />
              </motion.div>
            ) : (
              <motion.img
                className="full-player__cover"
                layoutId="now-playing-cover"
                src={cover}
                alt={`${item.title} 封面`}
                width={320}
                height={320}
                onError={() => setCoverFailed(true)}
              />
            )}
            <div className="full-player__meta">
              <h2 className="full-player__song" title={item.title}>
                {item.title}
              </h2>
              <p className="full-player__artist" title={item.author}>
                {item.author}
              </p>
            </div>

            <div className="full-player__progress">
              <ProgressBar
                currentTime={currentTime}
                duration={duration}
                onSeek={seek}
              />
            </div>

            <div className="full-player__controls">
              <IconButton
                icon={SkipBack}
                weight="fill"
                label="上一首"
                iconSize={26}
                onClick={() => void previous()}
              />
              <button
                type="button"
                className="icon-button full-player__play"
                aria-label={isPlaying ? "暂停" : "播放"}
                aria-busy={loadingStream}
                onClick={togglePlay}
              >
                <AnimatePresence mode="wait" initial={false}>
                  <motion.span
                    key={isPlaying ? "pause" : "play"}
                    className="full-player__play-icon"
                    initial={{ scale: 0.6, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    exit={{ scale: 0.6, opacity: 0 }}
                    transition={{ duration: 0.15 }}
                  >
                    {isPlaying ? (
                      <Pause size={30} weight="fill" aria-hidden />
                    ) : (
                      <Play size={30} weight="fill" aria-hidden />
                    )}
                  </motion.span>
                </AnimatePresence>
              </button>
              <IconButton
                icon={SkipForward}
                weight="fill"
                label="下一首"
                iconSize={26}
                onClick={() => void next()}
              />
            </div>

            <div className="full-player__extras">
              {(item.source === "bili" || item.source === "netease") && (
                <motion.span
                  key={fav ? "fav" : "unfav"}
                  className="full-player__heart-wrap"
                  animate={fav ? { scale: [1, 1.25, 1] } : { scale: 1 }}
                  transition={{ duration: 0.3, ease: "easeOut" }}
                >
                  <IconButton
                    icon={Heart}
                    weight={fav ? "fill" : "regular"}
                    label={fav ? "取消收藏" : "收藏"}
                    className={fav ? "full-player__heart--active" : ""}
                    onClick={() => {
                      if (item.source === "netease") {
                        if (fav) {
                          void removeNeteaseFavorite(item.cid).catch(
                            (error: unknown) => showToast(String(error)),
                          );
                          showToast("已取消收藏");
                        } else {
                          void addNeteaseFavorite(
                            queueItemToNeteaseSong(item),
                          ).catch((error: unknown) =>
                            showToast(String(error)),
                          );
                          showToast("已收藏");
                        }
                        return;
                      }
                      if (fav) {
                        void removeFavorite(item.bvid, item.cid).catch((error: unknown) =>
                          showToast(String(error)),
                        );
                        showToast("已取消收藏");
                      } else {
                        void addFavorite(queueItemToTrackInfo(item)).catch(
                          (error: unknown) => showToast(String(error)),
                        );
                        showToast("已收藏");
                      }
                    }}
                  />
                </motion.span>
              )}
              {item.source === "bili" && (
                <IconButton
                  icon={ListPlus}
                  label="添加到歌单"
                  onClick={() => openPicker(queueItemToTrackInfo(item))}
                />
              )}
              <div className="full-player__volume">
                {volume <= 0.05 ? (
                  <SpeakerLow size={16} aria-hidden />
                ) : (
                  <SpeakerHigh size={16} aria-hidden />
                )}
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.01}
                  value={volume}
                  aria-label="音量"
                  onChange={(event) => setVolume(Number(event.target.value))}
                />
              </div>
                <IconButton
                  icon={Queue}
                  label="打开播放队列"
                  onClick={toggleQueuePanel}
                />

              {streamId && cachePercent > 0 && (
                <span className="full-player__cache">
                  缓存 {cachePercent}%
                </span>
              )}
            </div>
              </div>
              {item.source === "netease" && (
                <aside className="full-player__lyrics" aria-label="歌词" ref={lyricsViewportRef}>
                  {lyricLoading ? (
                    <p className="full-player__lyrics-state">歌词加载中…</p>
                  ) : lyricError ? (
                    <p className="full-player__lyrics-state">歌词加载失败</p>
                  ) : lyricLines.length === 0 ? (
                    <p className="full-player__lyrics-state">暂无歌词</p>
                  ) : (
                    <div className="full-player__lyrics-list" ref={lyricsTrackRef}>
                      {lyricLines.map((line, index) => (
                        <div
                          key={`${line.time}-${index}`}
                            role="button"
                            tabIndex={0}
                            onClick={() => seek(line.time)}
                            onKeyDown={(event) => {
                              if (event.key === "Enter" || event.key === " ") {
                                event.preventDefault();
                                seek(line.time);
                              }
                            }}

                          ref={
                            index === activeLyricIndex
                              ? activeLyricRef
                              : undefined
                          }
                          className={`full-player__lyric-line${
                            index === activeLyricIndex
                              ? " full-player__lyric-line--active"
                              : ""
                          }`}
                        >
                          <span className="full-player__lyric-text">
                            {line.text || "…"}
                          </span>
                          {line.translation && (
                            <span className="full-player__lyric-translation">
                              {line.translation}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </aside>
              )}

          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
