import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, MusicNotes, WarningCircle } from "@phosphor-icons/react";
import { CoverImage } from "../components/common/CoverImage";
import { usePlayerStore } from "../stores/playerStore";
import { formatDuration } from "../lib/track";
import type {
  NeteaseArtistDetail,
  NeteaseSong,
} from "../types/models";

interface ArtistPageProps {
  artistId: number;
}

export function ArtistPage({ artistId }: ArtistPageProps) {
  const [detail, setDetail] = useState<NeteaseArtistDetail | null>(null);
  const [songs, setSongs] = useState<NeteaseSong[]>([]);
    const [visibleCount, setVisibleCount] = useState(10);

  
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const playNeteaseSong = usePlayerStore((state) => state.playNeteaseSong);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setDetail(null);
    setSongs([]);
    

    Promise.all([
      invoke<NeteaseArtistDetail>("get_netease_artist_detail", { artistId }),
      invoke<NeteaseSong[]>("get_netease_artist_songs", { artistId }),
      
    ])
      .then(([detailResult, songResult]) => {
        if (cancelled) return;
        setDetail(detailResult);
        setSongs(songResult);
        setVisibleCount(10);
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
  }, [artistId]);

    useEffect(() => {
      const container = document.getElementById("main-content");
      if (!container) return;

      const handleScroll = () => {
        if (
          container.scrollTop + container.clientHeight >=
          container.scrollHeight - 120
        ) {
          setVisibleCount((count) =>
            Math.min(count + 10, songs.length),
          );
        }
      };

      container.addEventListener("scroll", handleScroll);
      return () => container.removeEventListener("scroll", handleScroll);
    }, [songs.length]);


  if (loading) {
    return (
      <section className="page artist-page">
        
        <h1>加载中…</h1>
      </section>
    );
  }

  if (error) {
    return (
      <section className="page artist-page">
        <button
          type="button"
          className="button button--ghost"
          onClick={() => {
            window.location.hash = "/search";
          }}
        >
          <ArrowLeft size={16} aria-hidden />
          返回搜索
        </button>
        <div className="search-state search-state--error">
          <WarningCircle size={40} aria-hidden />
          <p className="search-state__title">歌手页加载失败</p>
          <p className="search-state__hint">{error}</p>
        </div>
      </section>
    );
  }

  return (
    <section className="page artist-page">
      <button
        type="button"
        className="button button--ghost artist-page__back"
        onClick={() => {
          window.location.hash = "/search";
        }}
      >
        <ArrowLeft size={16} aria-hidden />
        返回搜索
      </button>

      <div className="artist-page__header">
        <CoverImage
          src={detail?.picUrl ?? ""}
          alt=""
          className="artist-page__avatar"
          width={120}
          height={120}
        />
        <div className="artist-page__info">
          <h1>{detail?.name ?? "未知歌手"}</h1>
          {detail?.description && (
            <p className="artist-page__desc">{detail.description}</p>
          )}
        </div>
      </div>

      <div className="artist-page__section">
        <h2 className="artist-page__section-title">热门歌曲</h2>
        {songs.length === 0 ? (
          <p className="artist-page__empty">暂无歌曲</p>
        ) : (
          <div className="artist-song-list">
            {songs.slice(0, visibleCount).map((song, index) => (
              <button
                type="button"
                key={song.id}
                className="artist-song-row"
                onClick={() => void playNeteaseSong(song)}
              >
                <span className="artist-song-row__index">{index + 1}</span>
                <CoverImage
                  src={song.picUrl}
                  alt=""
                  className="artist-song-row__cover"
                  width={48}
                  height={48}
                />
                <span className="artist-song-row__main">
                  <span className="artist-song-row__name" title={song.name}>
                    {song.name}
                  </span>
                  <span className="artist-song-row__album" title={song.albumName}>
                    {song.albumName || "未知专辑"}
                  </span>
                </span>
                <span className="artist-song-row__duration">
                  {formatDuration(song.durationMs / 1000)}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>

        {/*

      <div className="artist-page__section">
        <h2 className="artist-page__section-title">专辑</h2>
        {albums.length === 0 ? (
          <p className="artist-page__empty">暂无专辑</p>
        ) : (
          <div className="artist-album-grid">
            {albums.map((album) => (
              
                <div
                  className="artist-album-card"
                  key={album.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => {
                    window.location.hash = `/album/${album.id}`;
                  }}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      window.location.hash = `/album/${album.id}`;
                    }
                  }}
                >
                <CoverImage
                  src={album.picUrl}
                  alt=""
                  className="artist-album-card__cover"
                  width={160}
                  height={160}
                />
                <span className="artist-album-card__name" title={album.name}>
                  {album.name}
                </span>
                <span className="artist-album-card__meta">
                  {album.size} 首
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
        */}

      <div className="artist-page__footer">
        <MusicNotes size={16} aria-hidden />
        SenTune · 网易云歌手
      </div>
    </section>
  );
}
