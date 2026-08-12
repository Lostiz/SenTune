import { useState } from "react";
import {
  ArrowLeft,
  CaretDown,
  CaretUp,
  Playlist,
  Plus,
  Trash,
} from "@phosphor-icons/react";
import { EmptyState } from "../components/common/EmptyState";
import { formatDuration } from "../lib/track";
import { useLibraryStore } from "../stores/libraryStore";
import { usePlayerStore } from "../stores/playerStore";
import type { PlaylistDetail, TrackInfo } from "../types/models";
import { CoverImage } from "../components/common/CoverImage";

function PlaylistRow({
  track,
  index,
  count,
  onPlay,
  onMoveUp,
  onMoveDown,
  onRemove,
  onMoveTo,
  dragging,
  onDragStart,
  onDragEnd,
}: {
  track: TrackInfo;
  index: number;
  count: number;
  onPlay: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onRemove: () => void;
  onMoveTo: (targetIndex: number) => void;
  dragging: boolean;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const [dragOver, setDragOver] = useState(false);
  return (
    <li
      className={`playlist-row${dragOver ? " playlist-row--drag-over" : ""}${
        dragging ? " playlist-row--dragging" : ""
      }`}
      draggable
      onDragStart={(event) => {
        event.dataTransfer.setData("text/plain", String(index));
        event.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      onDragOver={(event) => {
        event.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(event) => {
        event.preventDefault();
        setDragOver(false);
        const from = Number(event.dataTransfer.getData("text/plain"));
        if (Number.isFinite(from) && from !== index) onMoveTo(index);
      }}
    >
      <span className="playlist-row__index">{index + 1}</span>
      <button type="button" className="playlist-row__main" onClick={onPlay}>
        <CoverImage
          src={track.coverUrl}
          alt=""
          width={40}
          height={40}
        />
        <span className="playlist-row__text">
          <span className="playlist-row__name" title={track.title}>
            {track.title}
          </span>
          <span className="playlist-row__meta">
            {track.author} · {formatDuration(track.duration)}
            {track.cachedAt !== null && " · 已缓存"}
          </span>
        </span>
      </button>
      <div className="playlist-row__actions">
        <button
          type="button"
          className="icon-button"
          aria-label={`上移 ${track.title}`}
          disabled={index === 0}
          onClick={onMoveUp}
        >
          <CaretUp size={16} aria-hidden />
        </button>
        <button
          type="button"
          className="icon-button"
          aria-label={`下移 ${track.title}`}
          disabled={index === count - 1}
          onClick={onMoveDown}
        >
          <CaretDown size={16} aria-hidden />
        </button>
        <button
          type="button"
          className="icon-button"
          aria-label={`从歌单移除 ${track.title}`}
          onClick={onRemove}
        >
          <Trash size={16} aria-hidden />
        </button>
      </div>
    </li>
  );
}

export function PlaylistsPage() {
  const playlists = useLibraryStore((state) => state.playlists);
  const openPlaylist = useLibraryStore((state) => state.openPlaylist);
  const createPlaylist = useLibraryStore((state) => state.createPlaylist);
  const deletePlaylist = useLibraryStore((state) => state.deletePlaylist);
  const renamePlaylist = useLibraryStore((state) => state.renamePlaylist);
  const removeFromPlaylist = useLibraryStore((state) => state.removeFromPlaylist);
  const moveInPlaylist = useLibraryStore((state) => state.moveInPlaylist);
  const playItem = usePlayerStore((state) => state.playItem);
  const [detail, setDetail] = useState<PlaylistDetail | null>(null);
  const [newName, setNewName] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [draggingIndex, setDraggingIndex] = useState<number | null>(null);

  const openDetail = async (id: number) => {
    const next = await openPlaylist(id);
    if (next) setDetail(next);
  };

  const refreshDetail = async (id: number) => {
    const next = await openPlaylist(id);
    if (next) setDetail(next);
  };

  if (detail) {
    return (
      <section className="page">
        <div className="page__header">
          <button
            type="button"
            className="button button--ghost"
            onClick={() => setDetail(null)}
          >
            <ArrowLeft size={16} aria-hidden />
            返回
          </button>
          {editingName === null ? (
            <h1
              className="playlist-detail__title"
              title="点击重命名"
              tabIndex={0}
              role="button"
              onClick={() => {
                setEditingName(detail.name);
                setRenameValue(detail.name);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  setEditingName(detail.name);
                  setRenameValue(detail.name);
                }
              }}
            >
              {detail.name}
            </h1>
          ) : (
            <form
              className="playlist-detail__rename"
              onSubmit={async (event) => {
                event.preventDefault();
                if (renameValue.trim()) {
                  await renamePlaylist(detail.id, renameValue.trim());
                  setEditingName(null);
                  void refreshDetail(detail.id);
                }
              }}
            >
              <input
                type="text"
                value={renameValue}
                onChange={(event) => setRenameValue(event.target.value)}
                aria-label="歌单名称"
                maxLength={40}
                autoFocus
              />
              <button type="submit" className="button button--primary">
                保存
              </button>
            </form>
          )}
          <button
            type="button"
            className={`button ${confirmDelete ? "button--danger" : "button--ghost"}`}
            onClick={async () => {
              if (!confirmDelete) {
                setConfirmDelete(true);
                return;
              }
              setConfirmDelete(false);
              await deletePlaylist(detail.id);
              setDetail(null);
            }}
          >
            <Trash size={16} aria-hidden />
            {confirmDelete ? "确认删除" : "删除歌单"}
          </button>
        </div>

        {detail.tracks.length === 0 ? (
          <EmptyState
            icon={Playlist}
            title="歌单为空"
            hint="在搜索卡片上点击 + 添加视频"
          />
        ) : (
          <ul className="playlist-detail__list">
            {detail.tracks.map((track, index) => (
              <PlaylistRow
                key={track.bvid}
                track={track}
                index={index}
                count={detail.tracks.length}
                onPlay={() => {
                  playItem({
                    source: "bili",
                    bvid: track.bvid,
                    cid: track.cid,
                    title: track.title,
                    cover: track.coverUrl,
                    author: track.author,
                    duration: track.duration,
                  });
                }}
                onMoveUp={() => {
                  void moveInPlaylist(detail.id, track.bvid, track.cid, index - 1).then(() =>
                    refreshDetail(detail.id),
                  );
                }}
                onMoveDown={() => {
                  void moveInPlaylist(detail.id, track.bvid, track.cid, index + 1).then(() =>
                    refreshDetail(detail.id),
                  );
                }}
                onRemove={() => {
                  void removeFromPlaylist(detail.id, track.bvid, track.cid).then(() =>
                    refreshDetail(detail.id),
                  );
                }}
                onMoveTo={(target) => {
                  void moveInPlaylist(detail.id, track.bvid, track.cid, target).then(() =>
                    refreshDetail(detail.id),
                  );
                }}
                dragging={draggingIndex === index}
                onDragStart={() => setDraggingIndex(index)}
                onDragEnd={() => setDraggingIndex(null)}
              />
            ))}
          </ul>
        )}
      </section>
    );
  }

  return (
    <section className="page">
      <p className="page__subtitle">歌单</p>
      <h1>歌单</h1>
      <form
        className="playlists__create"
        onSubmit={async (event) => {
          event.preventDefault();
          if (newName.trim()) {
            await createPlaylist(newName.trim());
            setNewName("");
          }
        }}
      >
        <label htmlFor="playlist-name" className="visually-hidden">
          新歌单名称
        </label>
        <input
          id="playlist-name"
          type="text"
          value={newName}
          onChange={(event) => setNewName(event.target.value)}
          placeholder="新歌单名称…"
          maxLength={40}
        />
        <button
          type="submit"
          className="button button--primary"
          disabled={!newName.trim()}
        >
          <Plus size={16} aria-hidden />
          创建歌单
        </button>
      </form>

      {playlists.length === 0 ? (
        <EmptyState
          icon={Playlist}
          title="还没有歌单"
          hint="创建歌单后，可在搜索卡片上添加视频"
        />
      ) : (
        <ul className="playlists__list">
          {playlists.map((playlist) => (
            <li key={playlist.id}>
              <button
                type="button"
                className="playlists__item"
                onClick={() => void openDetail(playlist.id)}
              >
                <Playlist size={22} aria-hidden />
                <span className="playlists__name">{playlist.name}</span>
                <span className="playlists__count">
                  {playlist.trackCount} 首
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
