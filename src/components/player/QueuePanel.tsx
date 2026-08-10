import { Queue } from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect } from "react";
import { usePlayerStore } from "../../stores/playerStore";
import { EmptyState } from "../common/EmptyState";
import { IconButton } from "../common/IconButton";
import { CoverImage } from "../common/CoverImage";

export function QueuePanel() {
  const open = usePlayerStore((state) => state.queuePanelOpen);
  const toggleQueuePanel = usePlayerStore((state) => state.toggleQueuePanel);
  const queue = usePlayerStore((state) => state.queue);
  const currentIndex = usePlayerStore((state) => state.currentIndex);
  const playAt = usePlayerStore((state) => state.playAt);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") toggleQueuePanel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, toggleQueuePanel]);

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            className="queue-panel__backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            onClick={toggleQueuePanel}
            aria-hidden
          />
          <motion.aside
            className="queue-panel"
            role="dialog"
            aria-label="播放队列"
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ duration: 0.25, ease: [0.25, 1, 0.5, 1] }}
          >
            <div className="queue-panel__header">
              <h2 className="queue-panel__title">播放队列</h2>
              <IconButton
                icon={Queue}
                label="关闭播放队列"
                onClick={toggleQueuePanel}
              />
            </div>
            {queue.length === 0 ? (
              <EmptyState
                icon={Queue}
                title="队列为空"
                hint="从搜索或首页选择视频开始播放"
              />
            ) : (
              <ul className="queue-panel__list">
                {queue.map((item, index) => (
                  <li key={`${item.bvid}-${index}`}>
                    <button
                      type="button"
                      className={`queue-panel__item${
                        index === currentIndex
                          ? " queue-panel__item--current"
                          : ""
                      }`}
                      onClick={() => void playAt(index)}
                      aria-current={index === currentIndex ? "true" : undefined}
                    >
                      <CoverImage
                        src={item.cover}
                        alt=""
                        width={40}
                        height={40}
                      />
                      <span className="queue-panel__text">
                        <span className="queue-panel__name" title={item.title}>
                          {item.title}
                        </span>
                        <span className="queue-panel__author">
                          {item.author}
                        </span>
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}
