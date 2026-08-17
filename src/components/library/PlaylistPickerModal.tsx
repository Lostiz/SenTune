import { useRef, useState } from "react";
import { ListPlus, Plus, X } from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import { useLibraryStore } from "../../stores/libraryStore";
import { usePlaylistPickerStore } from "../../stores/playlistPickerStore";
import { useToastStore } from "../../stores/toastStore";
import { IconButton } from "../common/IconButton";
import { useModalFocus } from "../../hooks/useModalFocus";

export function PlaylistPickerModal() {
  const { open, track, closePicker } = usePlaylistPickerStore();
  const playlists = useLibraryStore((state) => state.playlists);
  const createPlaylist = useLibraryStore((state) => state.createPlaylist);
  const addToPlaylist = useLibraryStore((state) => state.addToPlaylist);
  const showToast = useToastStore((state) => state.showToast);
  const [newName, setNewName] = useState("");
  const modalRef = useRef<HTMLDivElement>(null);
  useModalFocus(open, closePicker, modalRef);

  if (!open) return null;

  const handleAdd = async (playlistId: number) => {
    if (!track) return;
    try {
      await addToPlaylist(playlistId, track);
      showToast(`已添加到歌单：${track.title}`);
      closePicker();
    } catch (error) {
      showToast(String(error));
    }
  };

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name || !track) return;
    const id = await createPlaylist(name);
    if (id !== null) {
      await handleAdd(id);
      setNewName("");
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="modal-overlay"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.18 }}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) closePicker();
          }}
        >
          <motion.div
            ref={modalRef}
            className="modal playlist-picker"
            role="dialog"
            aria-modal="true"
            aria-label="添加到歌单"
            initial={{ opacity: 0, scale: 0.94, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.94, y: 8 }}
            transition={{ duration: 0.2, ease: [0.25, 1, 0.5, 1] }}
          >
            <div className="modal__header">
              <h2 className="modal__title">添加到歌单</h2>
              <IconButton
                icon={X}
                label="关闭添加到歌单弹窗"
                onClick={closePicker}
              />
            </div>
            <div className="playlist-picker__body">
              {track && (
                <p className="playlist-picker__track" title={track.title}>
                  {track.title}
                </p>
              )}
              <ul className="playlist-picker__list">
                {playlists.map((playlist) => (
                  <li key={playlist.id}>
                    <button
                      type="button"
                      className="playlist-picker__item"
                      onClick={() => void handleAdd(playlist.id)}
                    >
                      <ListPlus size={18} aria-hidden />
                      <span className="playlist-picker__name">{playlist.name}</span>
                      <span className="playlist-picker__count">
                        {playlist.trackCount}
                      </span>
                    </button>
                  </li>
                ))}
                {playlists.length === 0 && (
                  <li className="playlist-picker__empty">还没有歌单，先创建一个</li>
                )}
              </ul>
              <form
                className="playlist-picker__create"
                onSubmit={(event) => {
                  event.preventDefault();
                  void handleCreate();
                }}
              >
                <label htmlFor="new-playlist-name" className="visually-hidden">
                  新歌单名称
                </label>
                <input
                  id="new-playlist-name"
                  type="text"
                  value={newName}
                  onChange={(event) => setNewName(event.target.value)}
                  placeholder="新建歌单…"
                  maxLength={40}
                />
                <button
                  type="submit"
                  className="button button--primary"
                  disabled={!newName.trim()}
                >
                  <Plus size={16} aria-hidden />
                  创建
                </button>
              </form>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
