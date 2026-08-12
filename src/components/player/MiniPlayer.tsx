import { useEffect, useRef, useState } from "react";
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
import { bindAudio } from "../../lib/audioController";
import { useResolvedCover } from "../common/CoverImage";
import { ScrollingText } from "../common/ScrollingText";

export function MiniPlayer() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const item = usePlayerStore((state) =>
    state.currentIndex === null ? null : state.queue[state.currentIndex],
  );
  const isPlaying = usePlayerStore((state) => state.isPlaying);
  const currentTime = usePlayerStore((state) => state.currentTime);
  const duration = usePlayerStore((state) => state.duration);
  const streamUrl = usePlayerStore((state) => state.streamUrl);
  const streamId = usePlayerStore((state) => state.streamId);
  const cachePercent = usePlayerStore((state) => state.cachePercent);
  const loadingStream = usePlayerStore((state) => state.loadingStream);
  const playbackError = usePlayerStore((state) => state.playbackError);
  const currentAudioId = usePlayerStore((state) => state.currentAudioId);
  const volume = usePlayerStore((state) => state.volume);
  const fullPlayerOpen = usePlayerStore((state) => state.fullPlayerOpen);
  const [localCover, setLocalCover] = useState<string | null>(null);

  const togglePlay = usePlayerStore((state) => state.togglePlay);
  const previous = usePlayerStore((state) => state.previous);
  const next = usePlayerStore((state) => state.next);
  const seek = usePlayerStore((state) => state.seek);
  const setCurrentTime = usePlayerStore((state) => state.setCurrentTime);
  const setDuration = usePlayerStore((state) => state.setDuration);
  const setPlaying = usePlayerStore((state) => state.setPlaying);
  const setPlaybackError = usePlayerStore((state) => state.setPlaybackError);
  const retryPlay = usePlayerStore((state) => state.retryPlay);
  const setVolume = usePlayerStore((state) => state.setVolume);
  const openFullPlayer = usePlayerStore((state) => state.openFullPlayer);
  const toggleQueuePanel = usePlayerStore((state) => state.toggleQueuePanel);
  const isFavorite = useLibraryStore((state) => state.isFavorite);
  const addFavorite = useLibraryStore((state) => state.addFavorite);
  const removeFavorite = useLibraryStore((state) => state.removeFavorite);
  const openPicker = usePlaylistPickerStore((state) => state.openPicker);
  const showToast = useToastStore((state) => state.showToast);
  const { url: remoteCover, failed: coverFailed, setFailed: setCoverFailed } =
    useResolvedCover(item?.cover);

  useEffect(() => {
    setLocalCover(null);
    if (!item || item.source === "local") return;
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
    bindAudio(audioRef.current);
    return () => bindAudio(null);
  }, []);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !streamUrl) return;
    audio.src = streamUrl;
    audio.load();
    audio
      .play()
      .then(() => setPlaying(true))
      .catch((error: unknown) => {
        if ((error as DOMException | null)?.name !== "AbortError") {
          setPlaybackError(String(error));
        }
      });
  }, [streamUrl, setPlaybackError, setPlaying]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    audio.volume = volume;
  }, [volume]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    const onTime = () => setCurrentTime(audio.currentTime);
    const onDuration = () => {
      if (Number.isFinite(audio.duration)) setDuration(audio.duration);
    };
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onEnded = () => next();
    const onError = () => {
      if (streamId) {
        const mediaError = audio.error;
        const notSupported =
          mediaError?.code === MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED;
        setPlaybackError(
          notSupported
            ? "音质不支持，正在切换…"
            : "播放出错，正在重新获取音频流…",
        );
        void retryPlay(notSupported ? (currentAudioId ?? undefined) : undefined);
      }
    };
    audio.addEventListener("timeupdate", onTime);
    audio.addEventListener("durationchange", onDuration);
    audio.addEventListener("play", onPlay);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("error", onError);
    return () => {
      audio.removeEventListener("timeupdate", onTime);
      audio.removeEventListener("durationchange", onDuration);
      audio.removeEventListener("play", onPlay);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("error", onError);
    };
  }, [
    streamId,
    setCurrentTime,
    setDuration,
    setPlaying,
    setPlaybackError,
    next,
    retryPlay,
    currentAudioId,
  ]);

  const cover = localCover ?? remoteCover ?? item?.cover;
  const fav =
    item && item.source === "bili" ? isFavorite(item.bvid, item.cid) : false;

  return (
    <footer
      className={`mini-player${fullPlayerOpen ? " mini-player--hidden" : ""}`}
      role="region"
      aria-label="迷你播放器"
    >
      <audio ref={audioRef} preload="auto" />

      {item && !fullPlayerOpen && (
        <>
          <div className="mini-player__left">
            <button
              type="button"
              className="mini-player__expand"
              onClick={openFullPlayer}
              aria-label="展开全屏播放器"
            >
              {coverFailed && !localCover ? (
                <motion.span
                  className="mini-player__cover cover-fallback"
                  layoutId="now-playing-cover"
                  aria-hidden
                >
                  <MusicNotes size={20} weight="light" />
                </motion.span>
              ) : (
                <motion.img
                  className="mini-player__cover"
                  layoutId="now-playing-cover"
                  src={cover}
                  alt=""
                  width={48}
                  height={48}
                  onError={() => setCoverFailed(true)}
                />
              )}
              <span className="mini-player__info">
                <ScrollingText text={item.title} className="mini-player__title" />
                <span className="mini-player__meta" title={item.author}>
                  {item.author}
                </span>
              </span>
            </button>
            {item.source === "bili" && (
              <motion.span
                key={fav ? "fav" : "unfav"}
                className="mini-player__heart-wrap"
                animate={fav ? { scale: [1, 1.25, 1] } : { scale: 1 }}
                transition={{ duration: 0.3, ease: "easeOut" }}
              >
                <IconButton
                  icon={Heart}
                  weight={fav ? "fill" : "regular"}
                  label={fav ? "取消收藏" : "收藏"}
                  iconSize={16}
                  className={fav ? "mini-player__heart--active" : ""}
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
            )}
          </div>

          <div className="mini-player__center">
            <div className="mini-player__controls">
              <IconButton
                icon={SkipBack}
                weight="fill"
                label="上一首"
                iconSize={18}
                onClick={() => void previous()}
              />
              <button
                type="button"
                className="icon-button mini-player__play"
                aria-label={isPlaying ? "暂停" : "播放"}
                aria-busy={loadingStream}
                onClick={togglePlay}
              >
                <AnimatePresence mode="wait" initial={false}>
                  <motion.span
                    key={isPlaying ? "pause" : "play"}
                    className="mini-player__play-icon"
                    initial={{ scale: 0.6, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    exit={{ scale: 0.6, opacity: 0 }}
                    transition={{ duration: 0.15 }}
                  >
                    {isPlaying ? (
                      <Pause size={34} weight="fill" aria-hidden />
                    ) : (
                      <Play size={34} weight="fill" aria-hidden />
                    )}
                  </motion.span>
                </AnimatePresence>
              </button>
              <IconButton
                icon={SkipForward}
                weight="fill"
                label="下一首"
                iconSize={18}
                onClick={() => void next()}
              />
            </div>
            <ProgressBar
              currentTime={currentTime}
              duration={duration}
              onSeek={seek}
            />
          </div>

          <div className="mini-player__right">
            <div className="mini-player__status" aria-live="polite">
              {loadingStream && <span>加载中…</span>}
              {!loadingStream &&
                cachePercent > 0 &&
                cachePercent < 100 &&
                streamId && <span>缓存 {cachePercent}%</span>}
            </div>
            <div className="mini-player__volume">
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
                style={{
                  backgroundImage: `linear-gradient(to right, var(--text-secondary) ${
                    volume * 100
                  }%, var(--hairline-strong) ${volume * 100}%)`,
                }}
                onChange={(event) => setVolume(Number(event.target.value))}
              />
            </div>
            <IconButton
              icon={Queue}
              label="打开播放队列"
              onClick={toggleQueuePanel}
            />
            {item.source === "bili" && (
              <IconButton
                icon={ListPlus}
                label="添加到歌单"
                onClick={() => openPicker(queueItemToTrackInfo(item))}
              />
            )}
          </div>
        </>
      )}

      {playbackError && !fullPlayerOpen && (
        <p className="mini-player__error" role="alert">
          {playbackError}
        </p>
      )}
    </footer>
  );
}
