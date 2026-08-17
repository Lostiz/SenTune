import { MagnifyingGlass } from "@phosphor-icons/react";
import { useSearchStore } from "../stores/searchStore";
import { useSourceStore } from "../stores/sourceStore";

export function HomePage() {
  const source = useSourceStore((state) => state.source);
  const setSource = useSourceStore((state) => state.setSource);

  const switchSource = (next: "bilibili" | "netease") => {
    setSource(next);
    // 切换音源后清空搜索结果，避免两个音源的结果混显。
    useSearchStore.getState().clear();
  };

  return (
    <section className="page home-page">
      
      <h1>首页</h1>

      <div className="source-switch" role="group" aria-label="音源切换">
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

      <div className="search-state">
        <MagnifyingGlass size={40} aria-hidden />
        <p className="search-state__title">选择音源后开始搜索</p>
        <p className="search-state__hint">
          当前音源：
          {source === "bilibili" ? "BILIBILI" : "网易云（128K 标准音质）"}
        </p>
        <a className="button button--primary" href="#/search">
          去搜索
        </a>
      </div>
    </section>
  );
}
