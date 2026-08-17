import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, MusicNotes, WarningCircle } from "@phosphor-icons/react";
import { CoverImage } from "../components/common/CoverImage";
import { usePlayerStore } from "../stores/playerStore";
import { formatDuration } from "../lib/track";
import type { NeteaseAlbumDetail } from "../types/models";

interface AlbumPageProps {
  albumId: number;
}

export function AlbumPage({ albumId }: AlbumPageProps) {
  const [detail, setDetail] = useState<NeteaseAlbumDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const playNeteaseSong = usePlayerStore((state) => state.playNeteaseSong);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setDetail(null);

    invoke<NeteaseAlbumDetail>("get_netease_album_detail", { albumId })
      .then((result) => {
        if (!cancelled) setDetail(result);
      })
      .catch((requestError: unknown) => {
        if (!cancelled) setError(String(requestError));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [albumId]);

  if (loading) {
    return (
      <section className="page album-page">
        <p className="page__subtitle">专辑</p>
        <h1>加载中…</h1>
      </section>
    );
  }

  if (error || !detail) {
    return (
      <section className="page album-page">
        <button
          type="button"
          className="button button--ghost"
          onClick={() => window.history.back()}
        >
          <ArrowLeft size={16} aria-hidden />
          返回
        </button>
        <div className="search-state search-state--error">
          <WarningCircle size={40} aria-hidden />
          <p className="search-state__title">专辑页加载失败</p>
          <p className="search-state__hint">{error ?? "专辑不存在"}</p>
        </div>
      </section>
    );
  }

  const { album, songs } = detail;

  return (
    <section className="page album-page">
      <button
        type="button"
        className="button button--ghost album-page__back"
        onClick={() => window.history.back()}
      >
        <ArrowLeft size={16} aria-hidden />
        返回
      </button>

      <div className="album-page__header">
        <CoverImage
          src={album.picUrl}
          alt=""
          className="album-page__cover"
          width={160}
          height={160}
        />
        <div className="album-page__info">
          <p className="page__subtitle">专辑</p>
          <h1>{album.name}</h1>
          <p className="album-page__meta">
            {album.artist} · {album.size} 首
          </p>
        </div>
      </div>

      <div className="album-page__section">
        <h2 className="album-page__section-title">歌曲列表</h2>
        {songs.length === 0 ? (
          <p className="album-page__empty">暂无歌曲</p>
        ) : (
          <div className="album-song-list">
            {songs.map((song, index) => (
              <button
                type="button"
                key={song.id}
                className="album-song-row"
                onClick={() => void playNeteaseSong(song)}
              >
                <span className="album-song-row__index">{index + 1}</span>
                <CoverImage
                  src={song.picUrl}
                  alt=""
                  className="album-song-row__cover"
                  width={48}
                  height={48}
                />
                <span className="album-song-row__main">
                  <span className="album-song-row__name" title={song.name}>
                    {song.name}
                  </span>
                  <span className="album-song-row__artist" title={song.artist}>
                    {song.artist || "未知歌手"}
                  </span>
                </span>
                <span className="album-song-row__duration">
                  {formatDuration(song.durationMs / 1000)}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="album-page__footer">
        <MusicNotes size={16} aria-hidden />
        SenTune · 网易云专辑
      </div>
    </section>
  );
}
