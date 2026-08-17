import { useEffect } from "react";
import { MagnifyingGlass, WarningCircle } from "@phosphor-icons/react";
import { motion } from "motion/react";
import { useSearchStore } from "../../stores/searchStore";
import { SearchRow } from "./SearchRow";
import { NeteaseSearchRow } from "./NeteaseSearchRow";
import { NeteaseArtistRow } from "./NeteaseArtistRow";
import { Pagination } from "./Pagination";

const PAGE_SIZE = 10;

export function SearchResults() {
  const {
    keyword,
    source,
    items,
    neteaseItems,
      neteaseArtists,

    page,
    total,
    totalPages,
    loading,
    error,
    goToPage,
    search,
  } = useSearchStore();

  useEffect(() => {
    document.getElementById("main-content")?.scrollTo({ top: 0 });
  }, [page]);

  const isNetease = source === "netease";
  const itemCount = isNetease ? neteaseItems.length : items.length;
    const hasResults = isNetease
      ? neteaseItems.length > 0 || neteaseArtists.length > 0
      : items.length > 0;

  if (!keyword && !loading) {
    return (
      <div className="search-state">
        <MagnifyingGlass size={40} aria-hidden />
        <p className="search-state__title">
          {isNetease ? "搜索网易云音乐" : "搜索 Bilibili 视频"}
        </p>
        <p className="search-state__hint">输入关键词后按回车搜索</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="search-list" aria-busy="true">
        {Array.from({ length: PAGE_SIZE }, (_, index) => (
          <div className="search-row search-row--skeleton" key={index}>
            <span className="skeleton-block search-row__cover" />
            <span className="search-row__skeleton-lines">
              <span className="skeleton-line" />
              <span className="skeleton-line" />
            </span>
          </div>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="search-state search-state--error">
        <WarningCircle size={40} aria-hidden />
        <p className="search-state__title">搜索失败</p>
        <p className="search-state__hint">{error}</p>
        <button
          type="button"
          className="button button--primary"
          onClick={() => void search(keyword, page)}
        >
          重试
        </button>
      </div>
    );
  }

  if (!hasResults) {
    return (
      <div className="search-state">
        <MagnifyingGlass size={40} aria-hidden />
        <p className="search-state__title">
          没有找到「{keyword}」{isNetease ? "相关歌曲" : "相关视频"}
        </p>
        <p className="search-state__hint">换个关键词试试</p>
      </div>
    );
  }

  return (
    <>
        {isNetease && neteaseArtists.length > 0 && (
          <section className="search-artist-section" aria-label="歌手">
            <h2 className="search-artist-section__title">歌手</h2>
            <div className="search-list">
              {neteaseArtists.map((artist, index) => (
                <motion.div
                  key={artist.id}
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{
                    delay: Math.min(index, 5) * 0.04,
                    duration: 0.25,
                    ease: "easeOut",
                  }}
                >
                  <NeteaseArtistRow artist={artist} eager={index < 3} />
                </motion.div>
              ))}
            </div>
          </section>
        )}

      <div className="search-list">
        {isNetease
          ? neteaseItems.map((item, index) => (
              <motion.div
                key={item.id}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  delay: Math.min(index, 8) * 0.04,
                  duration: 0.25,
                  ease: "easeOut",
                }}
              >
                <NeteaseSearchRow item={item} eager={index < 5} />
              </motion.div>
            ))
          : items.map((item, index) => (
              <motion.div
                key={item.bvid}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  delay: Math.min(index, 8) * 0.04,
                  duration: 0.25,
                  ease: "easeOut",
                }}
              >
                <SearchRow item={item} eager={index < 5} />
              </motion.div>
            ))}
      </div>
      {itemCount > 0 && (
        <div className="search-results__footer">
        <p className="search-results__meta" aria-live="polite">
          共 {total} 条 · 第 {page}/{totalPages} 页
        </p>
        <Pagination
          page={page}
          totalPages={totalPages}
          onPageChange={goToPage}
        />
      </div>
        )}

    </>
  );
}
