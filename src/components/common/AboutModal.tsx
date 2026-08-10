import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import {
  AppWindow,
  Copy,
  FolderSimple,
  GithubLogo,
  MusicNotes,
  Rocket,
  Scroll,
  ShieldCheck,
  X,
} from "@phosphor-icons/react";
import type { Icon } from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import type { Variants } from "motion/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useAboutStore } from "../../stores/aboutStore";
import { useModalFocus } from "../../hooks/useModalFocus";
import { useToastStore } from "../../stores/toastStore";

const REPO_URL = "https://github.com/Lostiz/SenTune";
const MIT_LICENSE_URL = "https://choosealicense.com/licenses/mit/";

const staggerContainer: Variants = {
  hidden: {},
  show: {
    transition: { staggerChildren: 0.06, delayChildren: 0.08 },
  },
};

const staggerItem: Variants = {
  hidden: { opacity: 0, y: 12 },
  show: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.38, ease: [0.25, 0.46, 0.45, 0.94] },
  },
};

const techStack: { name: string; color: string; Icon: Icon }[] = [
  { name: "Rust", color: "#DE5623", Icon: ShieldCheck },
  { name: "Tauri 2", color: "#24C8DB", Icon: AppWindow },
];

export function AboutModal() {
  const open = useAboutStore((state) => state.open);
  const closeAbout = useAboutStore((state) => state.closeAbout);
  const showToast = useToastStore((state) => state.showToast);
  const modalRef = useRef<HTMLDivElement>(null);
  useModalFocus(open, closeAbout, modalRef);

  const [version, setVersion] = useState("1.0.0");
  const [dataDir, setDataDir] = useState("");

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void import("@tauri-apps/api/app")
      .then(async (mod) => {
        const appVersion = await mod.getVersion();
        if (!cancelled) setVersion(appVersion);
      })
      .catch(() => undefined);
    void import("@tauri-apps/api/path")
      .then(async (mod) => {
        const dir = await mod.appDataDir();
        if (!cancelled) setDataDir(dir);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [open]);

  const openExternal = (url: string) => {
    void openUrl(url).catch(() => {
      window.open(url, "_blank");
    });
  };

  const copyText = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast(`${label}已复制`);
      return;
    } catch {
      // WebView2 下兜底：隐藏 textarea + execCommand
    }
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch {
      copied = false;
    }
    textarea.remove();
    showToast(copied ? `${label}已复制` : "复制失败");
  };

  const links: {
    key: string;
    label: string;
    Icon: Icon;
    onClick: () => void;
  }[] = [
    {
      key: "github",
      label: "GitHub",
      Icon: GithubLogo,
      onClick: () => openExternal(REPO_URL),
    },
    {
      key: "license",
      label: "开源许可",
      Icon: Scroll,
      onClick: () => openExternal(MIT_LICENSE_URL),
    },
    {
      key: "data-dir",
      label: "数据目录",
      Icon: FolderSimple,
      onClick: () => {
        if (dataDir) {
          void copyText(dataDir, "数据目录");
        } else {
          showToast("数据目录暂不可用");
        }
      },
    },
    {
      key: "releases",
      label: "更新日志",
      Icon: Rocket,
      onClick: () => openExternal(REPO_URL),
    },
  ];

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="modal-overlay"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.18 }}
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) closeAbout();
          }}
        >
          <motion.div
            ref={modalRef}
            className="about-modal"
            role="dialog"
            aria-modal="true"
            aria-label="关于 SenTune"
            initial={{ opacity: 0, scale: 0.94, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{
              opacity: 0,
              scale: 0.94,
              y: 8,
              transition: { duration: 0.15, ease: [0.3, 0, 0.8, 0.15] },
            }}
            transition={{ duration: 0.2, ease: [0.25, 1, 0.5, 1] }}
          >
            <motion.div
              variants={staggerContainer}
              initial="hidden"
              animate="show"
            >
              <button
                type="button"
                className="about-modal__close"
                aria-label="关闭关于窗口"
                onClick={closeAbout}
              >
                <X size={14} aria-hidden />
              </button>

              <motion.div variants={staggerItem} className="about-logo" aria-hidden>
                <MusicNotes size={48} weight="fill" />
              </motion.div>

              <motion.h1 variants={staggerItem} className="about-title">
                Sen<span className="about-title__accent">Tune</span>
              </motion.h1>

              <motion.div variants={staggerItem} className="about-versions">
                <button
                  type="button"
                  className="about-version-badge"
                  aria-label={`复制应用版本 v${version}`}
                  onClick={() => void copyText(`SenTune v${version}`, "版本号")}
                >
                  <span className="about-version-badge__label">应用版本</span>
                  <span className="about-version-badge__value">v{version}</span>
                  <Copy
                    className="about-version-badge__copy"
                    size={14}
                    aria-hidden
                  />
                </button>
              </motion.div>

              <motion.p variants={staggerItem} className="about-description">
                轻量的网络音乐播放器
              </motion.p>

              <motion.div variants={staggerItem} className="about-section-label">
                Tech Stack
              </motion.div>

              <motion.div variants={staggerItem} className="about-tags">
                {techStack.map(({ name, color, Icon: TechIcon }) => (
                  <span
                    key={name}
                    className="about-tag"
                    style={{ "--tag-color": color } as CSSProperties}
                  >
                    <TechIcon size={12} weight="fill" aria-hidden />
                    {name}
                  </span>
                ))}
              </motion.div>

              <motion.div variants={staggerItem} className="about-links">
                {links.map(({ key, label, Icon: LinkIcon, onClick }) => (
                  <button
                    key={key}
                    type="button"
                    className="about-link-card"
                    onClick={onClick}
                  >
                    <LinkIcon size={16} weight="bold" aria-hidden />
                    <span>{label}</span>
                  </button>
                ))}
              </motion.div>

              <motion.footer variants={staggerItem} className="about-footer">
                <span>
                  Developed by{" "}
                  <button
                    type="button"
                    className="about-footer__link"
                    onClick={() => openExternal(REPO_URL)}
                  >
                    Lostiz
                  </button>{" "}
                  · MIT License
                </span>
                <span>© {new Date().getFullYear()} Lostiz</span>
              </motion.footer>
            </motion.div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
