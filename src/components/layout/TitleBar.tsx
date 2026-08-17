import { Copy, Minus, Square, X } from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { IconButton } from "../common/IconButton";

function run(action: () => Promise<void>) {
  void action().catch((error: unknown) => {
    console.error("窗口操作失败", error);
  });
}

export function TitleBar() {
  const appWindow = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="titlebar__title" data-tauri-drag-region>
        <span translate="no">SENTUNE</span>
      </div>
      <div className="titlebar__controls" data-tauri-drag-region>
        <IconButton
          icon={Minus}
          label="最小化"
          onClick={() => run(() => appWindow.minimize())}
        />
        <IconButton
          icon={Square}
          label="最大化"
          onClick={() => run(() => appWindow.toggleMaximize())}
        />
        <IconButton
          icon={Copy}
          label="还原"
          onClick={() => run(() => appWindow.unmaximize())}
        />
        <IconButton
          icon={X}
          label="关闭"
          onClick={() => run(() => appWindow.close())}
        />
      </div>
    </header>
  );
}
