import { useEffect, useState } from "react";
import { MusicNotes } from "@phosphor-icons/react";
import { coverProxyUrl } from "../../lib/proxy";

/** 解析封面为本地代理地址；失败时回退原地址，由 onError 处理占位。 */
export function useResolvedCover(src: string | undefined) {
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setFailed(false);
    setUrl(null);
    if (!src) {
      setFailed(true);
      return;
    }
    coverProxyUrl(src)
      .then((resolved) => {
        if (!cancelled) setUrl(resolved);
      })
      .catch(() => {
        if (!cancelled) setUrl(src);
      });
    return () => {
      cancelled = true;
    };
  }, [src]);

  return { url, failed, setFailed };
}

interface CoverImageProps {
  src: string;
  alt: string;
  className?: string;
  eager?: boolean;
  width?: number;
  height?: number;
}

/** 统一封面组件：走本地代理加载，失败显示占位图。 */
export function CoverImage({
  src,
  alt,
  className = "",
  eager = false,
  width,
  height,
}: CoverImageProps) {
  const { url, failed, setFailed } = useResolvedCover(src);

  if (failed || !url) {
    return (
      <span
        className={`cover-fallback ${className}`.trim()}
        role="img"
        aria-label={alt || "封面"}
        style={width || height ? { width, height } : undefined}
      >
        <MusicNotes size={28} weight="light" aria-hidden />
        <span className="cover-fallback__text">封面加载失败</span>
      </span>
    );
  }

  return (
    <img
      src={url}
      alt={alt}
      className={className}
      width={width}
      height={height}
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      onError={() => setFailed(true)}
    />
  );
}
