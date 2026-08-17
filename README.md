# SenTune

SenTune 是一个轻量 Windows 桌面音乐播放器：搜索 Bilibili 视频、按需抓取音频流、边下边播并缓存到本地，同时支持导入本地音乐文件夹，界面为 Apple Music 风格。支持合辑（多 P）逐集播放、本地音乐、收藏、歌单、播放历史、离线播放与深/浅色主题。

> 本项目仅供个人学习与本地缓存使用，不提供导出/分享功能，不处理大会员与付费内容。

## 功能特性

- **搜索**：B 站视频搜索
- **播放**：点击即播，按“可播放优先”选择音质（AAC 30216 → 30232 → 30280，无 AAC 才选 opus），自动过滤 Dolby / Hi-Res / 8D 等特殊音质。
- **边下边播**：本地 `127.0.0.1` 随机端口流代理，后台分块下载到 `.part`，播放器直接读取增长文件，支持拖动 seek 与远距离跳转。
- **CDN 自动切换**：主 CDN 节点不可达时自动使用 `backupUrl` 备用节点。
- **合辑（多 P）**：自动识别多 P 视频，把每一集展开到播放队列，支持上一首/下一首逐集切换。
- **缓存与离线**：整首播完后 `.part` 原子重命名为正式缓存文件；重启后可离线播放已缓存曲目。
- **资料库**：收藏、歌单（拖拽/上下按钮排序）、播放历史（同曲目去重）、离线标识。
- **本地音乐**：导入本地音乐文件夹，自动扫描并解析标题、艺术家、专辑、时长与封面；支持搜索、收藏、最近播放和本地播放。
- **封面**：统一通过本地代理加载（带 UA/Referer），固定尺寸，加载失败显示占位图。
- **设置**：深/浅色主题、缓存保留天数、容量上限（0 不限 / 最低 5GB）、立即清理、缓存目录选择与自动迁移、打开缓存目录。
- **界面**：Apple Music 风格，底部三栏播放器、macOS 圆点窗口按钮、全屏播放器、队列面板、音源切换（网易云默认 / BILIBILI）。
- **关于**：版本信息、技术栈、GitHub / 更新日志 / 开源许可 / 数据目录入口。

## 技术栈

- 前端：React 18 + TypeScript + Vite + Tailwind CSS + Zustand + Motion
- 桌面壳：Tauri 2
- 后端：Rust（lofty 音频元数据解析、reqwest、rusqlite、tiny_http、tokio）

## 环境要求

### Windows

- Windows 10/11（x64 或 ARM64）
- WebView2 Runtime（安装包会自动补装，缺失时安装过程中联网下载）

### Linux

- 主流发行版（Debian / Ubuntu / Fedora / Arch 等）
- 运行 deb / rpm 包需要对应的系统依赖（webkit2gtk、GTK3、librsvg 等）；AppImage 自带大部分依赖
- 构建需要安装 webkit2gtk-4.1 等开发库（见下方开发环境）

### macOS

- macOS 11 及以上
- 使用系统自带 WebKit，无需额外运行时
- 未签名构建，首次打开需要右键“打开”

## 安装与运行

### 开发环境（Windows）

本项目使用 Rust `x86_64-pc-windows-gnu` 工具链，编译 C 依赖（ring / SQLite）需要 MinGW-w64：

```powershell
$env:Path = "C:\Users\Moon\.local\msys64\mingw64\bin;" + $env:Path
$env:CC_x86_64_pc_windows_gnu = "gcc"
$env:AR_x86_64_pc_windows_gnu = "ar"
$env:RANLIB_x86_64_pc_windows_gnu = "ranlib"
```

脚本已封装上述环境：

- `scripts\dev.ps1`：启动开发窗口（`npm run tauri dev`）
- `scripts\test.ps1`：运行 `cargo test`（含 windows-gnu 测试清单注入）
- `scripts\build.ps1`：`cargo build`
- `scripts\build-release-exe.ps1`：编译 release 版可执行文件
- `scripts\build-release.ps1`：生成 NSIS 安装包（`npm run tauri build -- --target x86_64-pc-windows-gnu`）
- `scripts\make-icon.ps1`：重新生成应用图标

联网测试：

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\test.ps1 -- --ignored --nocapture
```

### 开发环境（Linux）

需要 Node.js ≥ 20 与 Rust stable，并安装系统依赖：

Debian / Ubuntu：

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Fedora / Arch 等发行版请安装对应的 webkit2gtk-4.1、GTK3、openssl、librsvg 开发包。随后：

```bash
npm install
npm run tauri dev
```

### 开发环境（macOS）

需要 Node.js ≥ 20、Rust stable 与 Xcode Command Line Tools：

```bash
xcode-select --install
npm install
npm run tauri dev
```

## 使用说明

1. **搜索**：输入关键词后按回车（或点右侧“搜索”按钮）。
2. **播放**：点击结果行开始播放；合辑会自动展开为多集队列，用上一首/下一首切换。
3. **缓存**：播放过程中后台持续下载，播放完成自动落盘；重启后可离线播放。
4. **收藏 / 歌单 / 历史**：在播放器或列表里操作，重启后数据保留。
5. **设置**：主题、缓存策略、立即清理、目录迁移。
6. **音源**：首页可切换音源；当前默认网易云，也可切换 BILIBILI。
7. **本地音乐**：侧边栏进入“本地音乐”，导入文件夹后可搜索、播放、收藏；本地曲目与 B 站曲目共用播放器队列。

## 数据目录

`app_data_dir`（`%APPDATA%\com.sentune.app`）：

- `sentune.db`：B 站曲目、本地曲目、收藏、歌单、历史、设置
- `cache/`：音频缓存（`{bvid}_{cid}_{audio_id}.m4a|opus`）、`covers/` 封面与 `local-covers/` 本地封面
- `logs/`：滚动日志（`sentune.log`），保留最近 3 份

## 项目结构

```text
src/                    # React 前端
  components/           # layout / search / player / library / common / settings
  stores/               # player / search / library / settings / source / about / toast
  pages/                # Home / Search / Local / Favorites / History / Playlists / Settings / About
  lib/                  # 工具（track / proxy / audioController）
src-tauri/
  src/
    api/                # B 站接口（wbi 签名、设备指纹、票据、搜索、详情、playurl）
    local/              # 本地音乐扫描与元数据解析
    stream/             # 本地流代理 + 后台下载器 + 增长文件读取
    cache/              # 缓存路径、封面、清理
    db/                 # SQLite（tracks / local / favorites / playlists / history / settings）
    commands/           # Tauri 命令层
    logging.rs          # 滚动日志
scripts/                # 开发 / 构建 / 测试 / 图标脚本
```

## 致谢
- [tauri-apps/api](https://github.com/tauri-apps/api)
- [phosphor-icons/react](https://github.com/phosphor-icons/react)
- [AnInsomniacy/motrix-next](https://github.com/AnInsomniacy/motrix-next)
- [NeteaseCloudMusicApi](https://github.com/Binaryify/NeteaseCloudMusicApi)（网易云公开接口参考）

## 常见问题

- **搜索触发风控（v_voucher）**：B 站对高频搜索有限制，请降低搜索频率，出现提示后稍等再试
- **“音质不支持”提示**：通常是首次播放冷启动或 CDN 节点切换的竞争，自动重试或稍后重试即可
- **找不到 WebView2Loader.dll**：请使用新版安装包，安装目录已内置该 DLL，并会在安装时自动补装 WebView2 Runtime。
- **歌单暂不支持本地曲目**：当前歌单仅支持 B 站曲目，本地曲目可在“本地音乐”页收藏与播放。

## 免责声明

- 音频仅用于个人本地缓存，不提供导出/分享功能。
- 不处理大会员、付费内容与歌词。
- 本项目与哔哩哔哩、网易云无任何关联。
- 本项目仅供学习使用，请尊重版权，请勿利用此项目从事商业行为或进行破坏版权行为。

## 开源许可

本项目基于 [MIT License](LICENSE) 开源。
