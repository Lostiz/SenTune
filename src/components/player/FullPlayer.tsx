import { useEffect, useState } from "react";
import {
  Heart,
  ListPlus,
  Pause,
  Play,
  Queue,
  SkipBack,
  SkipForward,
  SpeakerHigh,
  SpeakerLow,
  X,
  MusicNotes,
} from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import { invoke } from "@tauri-apps/api/core";
import { useLibraryStore } from "../../stores/libraryStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { usePlayerStore } from "../../stores/playerStore";
import { useToastStore } from "../../stores/toastStore";
import { IconButton } from "../common/IconButton";
import { ProgressBar } from "./ProgressBar";
import { queueItemToTrackInfo } from "../../lib/track";
import { useResolvedCover } from "../common/CoverImage";

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
  const openPicker = usePlaylistPickerStore((state) => state.openPicker);
  const showToast = useToastStore((state) => state.showToast);
  const {
    url: remoteCover,
    failed: coverFailed,
    setFailed: setCoverFailed,
  } = useResolvedCover(item?.cover);
  const [localCover, setLocalCover] = useState<string | null>(null);

  useEffect(() => {
    setLocalCover(null);
    if (!item) return;
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
  const fav = item ? isFavorite(item.bvid, item.cid) : false;

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

          <div className="full-player__top">
            <IconButton
              icon={X}
              label="收起全屏播放器"
              onClick={closeFullPlayer}
            />
            <span className="full-player__title" translate="no">
              SenTune
            </span>
            <IconButton
              icon={Queue}
              label="打开播放队列"
              onClick={toggleQueuePanel}
            />
          </div>

          <div className="full-player__body">
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
              <IconButton
                icon={ListPlus}
                label="添加到歌单"
                onClick={() => openPicker(queueItemToTrackInfo(item))}
              />
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
              {streamId && cachePercent > 0 && (
                <span className="full-player__cache">
                  缓存 {cachePercent}%
                </span>
              )}
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
