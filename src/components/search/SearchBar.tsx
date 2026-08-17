import { useState } from "react";
import { CircleNotch, MagnifyingGlass } from "@phosphor-icons/react";
import { useSearchStore } from "../../stores/searchStore";
import { useSourceStore, type MusicSource } from "../../stores/sourceStore";

export function SearchBar() {
  const search = useSearchStore((state) => state.search);
  const clear = useSearchStore((state) => state.clear);
  const loading = useSearchStore((state) => state.loading);
  const source = useSourceStore((state) => state.source);
  const setSource = useSourceStore((state) => state.setSource);
  const [text, setText] = useState("");

  const commit = (value: string) => {
    if (value.trim()) {
      void search(value);
    } else {
      clear();
    }
  };

  const switchSource = (next: MusicSource) => {
    if (source === next) return;
    setSource(next);
    clear();
  };

  const isNetease = source === "netease";

  return (
    <div className="search-form" aria-busy={loading}>
      <div
        className="source-switch source-switch--search"
        role="group"
        aria-label="音源切换"
      >
        <button
          type="button"
          className={`source-switch__item${
            source === "bilibili" ? " source-switch__item--active" : ""
          }`}
          aria-pressed={source === "bilibili"}
          onClick={() => switchSource("bilibili")}
        >
          BILIBILI
        </button>
        <button
          type="button"
          className={`source-switch__item${
            source === "netease" ? " source-switch__item--active" : ""
          }`}
          aria-pressed={source === "netease"}
          onClick={() => switchSource("netease")}
        >
          网易云
        </button>
      </div>
      <div className="search-bar">
        <MagnifyingGlass
          size={18}
          className="search-bar__icon"
          aria-hidden
        />
        <label htmlFor="search-input" className="visually-hidden">
          {isNetease ? "搜索网易云音乐" : "搜索 Bilibili 视频"}
        </label>
        <input
          id="search-input"
          type="search"
          name="keyword"
          autoComplete="off"
          spellCheck={false}
          placeholder={
            isNetease
              ? "输入歌名/歌手后按回车搜索，如：晴天…"
              : "输入关键词后按回车搜索，如：纯音乐…"
          }
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              commit(text);
            }
          }}
        />
      </div>
      <button
        type="button"
        className="search-bar__submit"
        aria-label="搜索"
        disabled={loading}
        onClick={() => commit(text)}
      >
        {loading ? (
          <CircleNotch size={16} className="spin" aria-hidden />
        ) : (
          <MagnifyingGlass size={16} weight="bold" aria-hidden />
        )}
        <span>搜索</span>
      </button>
    </div>
  );
}
