import { MagnifyingGlass } from "@phosphor-icons/react";
import { useSourceStore } from "../stores/sourceStore";
import { useToastStore } from "../stores/toastStore";

export function HomePage() {
  const source = useSourceStore((state) => state.source);
  const setSource = useSourceStore((state) => state.setSource);
  const showToast = useToastStore((state) => state.showToast);

  return (
    <section className="page home-page">
      <p className="page__subtitle">音乐</p>
      <h1>首页</h1>

      <div className="source-switch" role="group" aria-label="音源切换">
        <button
          type="button"
          className={`source-switch__item${
            source === "bilibili" ? " source-switch__item--active" : ""
          }`}
          aria-pressed={source === "bilibili"}
          onClick={() => setSource("bilibili")}
        >
          BILIBILI
        </button>
        <button
          type="button"
          className={`source-switch__item${
            source === "netease" ? " source-switch__item--active" : ""
          }`}
          aria-pressed={source === "netease"}
          onClick={() => {
            // 网易云音源暂未接入，仅占位提示。
            showToast("网易云音源开发中，敬请期待");
          }}
        >
          网易云
        </button>
      </div>

      <div className="search-state">
        <MagnifyingGlass size={40} aria-hidden />
        <p className="search-state__title">选择音源后开始搜索</p>
        <p className="search-state__hint">
          当前音源：{source === "bilibili" ? "BILIBILI" : "网易云"}
        </p>
        <a className="button button--primary" href="#/search">
          去搜索
        </a>
      </div>
    </section>
  );
}
