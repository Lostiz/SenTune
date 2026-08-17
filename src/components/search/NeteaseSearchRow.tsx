import { usePlayerStore } from "../../stores/playerStore";
import { useToastStore } from "../../stores/toastStore";
import { formatDuration } from "../../lib/track";
import { CoverImage } from "../common/CoverImage";
import type { NeteaseSong } from "../../types/models";

interface NeteaseSearchRowProps {
  item: NeteaseSong;
  eager?: boolean;
}

export function NeteaseSearchRow({ item, eager = false }: NeteaseSearchRowProps) {
  const playNeteaseSong = usePlayerStore((state) => state.playNeteaseSong);
  const showToast = useToastStore((state) => state.showToast);

  return (
    <div className="search-row-wrap">
      <button
        type="button"
        className="search-row"
        onClick={() => {
          void playNeteaseSong(item);
          showToast(`正在播放：${item.name}`);
        }}
        aria-label={`播放 ${item.name}，歌手：${item.artist}`}
      >
        <CoverImage
          src={item.picUrl}
          alt=""
          className="search-row__cover"
          eager={eager}
          width={96}
          height={96}
        />
        <span className="search-row__info">
          <span className="search-row__title" title={item.name}>
            {item.name}
            <span className="search-row__badge">网易云</span>
            {item.fee !== 0 && (
              <span className="search-row__badge search-row__badge--vip">
                VIP
              </span>
            )}
          </span>
          <span className="search-row__meta">
            {item.artist || "未知歌手"} · {formatDuration(item.durationMs / 1000)} ·{" "}
            128K
          </span>
        </span>
      </button>
    </div>
  );
}
