import { useEffect, useState } from "react";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${Math.max(0, Math.round(bytes / 1024))} KB`;
}

export function SettingsPage() {
  const theme = useSettingsStore((state) => state.theme);
  const setTheme = useSettingsStore((state) => state.setTheme);
  const cache = useSettingsStore((state) => state.cache);
  const cacheStatus = useSettingsStore((state) => state.cacheStatus);
  const loadCacheSettings = useSettingsStore((state) => state.loadCacheSettings);
  const saveCacheSettings = useSettingsStore((state) => state.saveCacheSettings);
  const refreshCacheStatus = useSettingsStore((state) => state.refreshCacheStatus);
  const clearCache = useSettingsStore((state) => state.clearCache);
  const pickCacheDir = useSettingsStore((state) => state.pickCacheDir);
  const showToast = useToastStore((state) => state.showToast);
  const [keepDays, setKeepDays] = useState(7);
  const [capacityGb, setCapacityGb] = useState(0);
  const [confirmClean, setConfirmClean] = useState(false);

  useEffect(() => {
    void loadCacheSettings();
    void refreshCacheStatus();
  }, [loadCacheSettings, refreshCacheStatus]);

  useEffect(() => {
    setKeepDays(cache.keepDays);
    setCapacityGb(cache.capacityLimitGb);
  }, [cache.keepDays, cache.capacityLimitGb]);

  return (
    <section className="page settings-page">
      <p className="page__subtitle">设置</p>
      <h1>设置</h1>
      <div className="settings-card">
        <h2 className="settings-card__title">外观</h2>
        <div className="settings-row">
          <div className="settings-row__main">
            <p className="settings-row__label">主题</p>
            <p className="settings-row__hint">深色与浅色即时切换</p>
          </div>
          <div className="theme-switch" role="group" aria-label="主题切换">
            <button
              type="button"
              className={`button ${
                theme === "dark" ? "button--primary" : "button--ghost"
              }`}
              onClick={() => setTheme("dark")}
              aria-pressed={theme === "dark"}
            >
              深色
            </button>
            <button
              type="button"
              className={`button ${
                theme === "light" ? "button--primary" : "button--ghost"
              }`}
              onClick={() => setTheme("light")}
              aria-pressed={theme === "light"}
            >
              浅色
            </button>
          </div>
        </div>
      </div>

      <div className="settings-card">
        <h2 className="settings-card__title">缓存</h2>
        <div className="settings-row">
          <div className="settings-row__main">
            <p className="settings-row__label">缓存目录</p>
            <p className="settings-row__hint">
              {cache.cachePath ?? "默认：应用数据目录下的 cache 文件夹"}
            </p>
          </div>
          <button
            type="button"
            className="button button--ghost"
            onClick={async () => {
              const picked = await pickCacheDir();
              if (picked) {
                await saveCacheSettings({ cachePath: picked });
                await refreshCacheStatus();
                showToast("缓存目录已更新并迁移");
              }
            }}
          >
            选择目录
          </button>
        </div>
        <div className="settings-grid">
          <label className="settings-field">
            <span className="settings-field__label">保留天数</span>
            <input
              type="number"
              min={1}
              max={365}
              value={keepDays}
              onChange={(event) => setKeepDays(Number(event.target.value))}
              onBlur={() => {
                const value = Math.max(1, Math.min(365, keepDays || 7));
                setKeepDays(value);
                void saveCacheSettings({ keepDays: value }).then(() =>
                  showToast("保留天数已保存"),
                );
              }}
              aria-label="缓存保留天数"
            />
          </label>
          <label className="settings-field">
            <span className="settings-field__label">容量上限（GB）</span>
            <input
              type="number"
              min={0}
              step={1}
              value={capacityGb}
              onChange={(event) => setCapacityGb(Number(event.target.value))}
              onBlur={() => {
                const value =
                  capacityGb === 0 ? 0 : Math.max(5, capacityGb || 0);
                setCapacityGb(value);
                void saveCacheSettings({ capacityLimitGb: value }).then(() =>
                  showToast(value === 0 ? "容量上限：不限" : `容量上限：${value}GB`),
                );
              }}
              aria-label="缓存容量上限，0 表示不限"
            />
            <span className="settings-field__hint">0 表示不限，最低 5GB</span>
          </label>
        </div>
        <div className="settings-row settings-row--status">
          <div className="settings-row__main">
            <p className="settings-row__label">
              当前占用：{formatBytes(cacheStatus?.totalSize ?? 0)}
            </p>
            <p className="settings-row__hint">
              缓存文件 {cacheStatus?.fileCount ?? 0} 个
              {cacheStatus?.capacityLimitGb ? ` · 上限 ${cacheStatus.capacityLimitGb}GB` : " · 不设上限"}
            </p>
          </div>
          <button
            type="button"
            className={`button ${confirmClean ? "button--danger" : "button--ghost"}`}
            onClick={async () => {
              if (!confirmClean) {
                setConfirmClean(true);
                return;
              }
              setConfirmClean(false);
              const before = cacheStatus;
              const after = await clearCache(0);
              const removed =
                before && after ? before.fileCount - after.fileCount : 0;
              showToast(
                removed > 0
                  ? `已清理 ${removed} 个文件`
                  : "缓存清理完成（没有可清理的文件）",
              );
            }}
          >
            {confirmClean ? "确认清空缓存" : "立即清理"}
          </button>
        </div>
      </div>
    </section>
  );
}
