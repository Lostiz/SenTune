import { useState } from "react";
import { CircleNotch, MagnifyingGlass } from "@phosphor-icons/react";
import { useSearchStore } from "../../stores/searchStore";

export function SearchBar() {
  const search = useSearchStore((state) => state.search);
  const clear = useSearchStore((state) => state.clear);
  const loading = useSearchStore((state) => state.loading);
  const [text, setText] = useState("");

  const commit = (value: string) => {
    if (value.trim()) {
      void search(value);
    } else {
      clear();
    }
  };

  return (
    <div className="search-form" aria-busy={loading}>
      <div className="search-bar">
        <MagnifyingGlass
          size={18}
          className="search-bar__icon"
          aria-hidden
        />
        <label htmlFor="search-input" className="visually-hidden">
          搜索 Bilibili 视频
        </label>
        <input
          id="search-input"
          type="search"
          name="keyword"
          autoComplete="off"
          spellCheck={false}
          placeholder="输入关键词后按回车搜索，如：纯音乐…"
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
