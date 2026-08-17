import { CoverImage } from "../common/CoverImage";
import type { NeteaseArtist } from "../../types/models";

interface NeteaseArtistRowProps {
  artist: NeteaseArtist;
  eager?: boolean;
}

export function NeteaseArtistRow({ artist, eager = false }: NeteaseArtistRowProps) {
  return (
    <div className="search-row-wrap">
      <button
        type="button"
        className="search-row search-row--artist"
        onClick={() => {
          window.location.hash = `/artist/${artist.id}`;
        }}
        aria-label={`查看歌手 ${artist.name}`}
      >
        <CoverImage
          src={artist.picUrl}
          alt=""
          className="search-row__cover search-row__cover--artist"
          eager={eager}
          width={64}
          height={64}
        />
        <span className="search-row__info">
          <span className="search-row__title" title={artist.name}>
            {artist.name}
            <span className="search-row__badge">歌手</span>
          </span>
          <span className="search-row__meta">查看歌手主页</span>
        </span>
      </button>
    </div>
  );
}
