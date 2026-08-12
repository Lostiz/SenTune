import { useState } from "react";
import type { ComponentType } from "react";
import {
  ClockCounterClockwise,
  GearSix,
  Heart,
  House,
  Info,
  MagnifyingGlass,
  MusicNotes,
  Playlist,
} from "@phosphor-icons/react";
import type { IconProps } from "@phosphor-icons/react";
import { motion } from "motion/react";
import { Icon } from "../common/Icon";
import { useSearchStore } from "../../stores/searchStore";
import { useAboutStore } from "../../stores/aboutStore";

export type PageId =
  | "home"
  | "search"
  | "playlists"
  | "favorites"
  | "history"
  | "local"
  | "settings"
  | "about";

interface NavItem {
  id: PageId;
  label: string;
  icon: ComponentType<IconProps>;
}

const LIBRARY_ITEMS: NavItem[] = [
  { id: "home", label: "首页", icon: House },
  { id: "search", label: "搜索", icon: MagnifyingGlass },
  { id: "local", label: "本地音乐", icon: MusicNotes },
  { id: "favorites", label: "个人收藏", icon: Heart },
  { id: "history", label: "播放历史", icon: ClockCounterClockwise },
];

const MANAGE_ITEMS: NavItem[] = [
  { id: "playlists", label: "歌单", icon: Playlist },
  { id: "settings", label: "设置", icon: GearSix },
  { id: "about", label: "关于", icon: Info },
];

interface SidebarProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
}

function NavList({
  items,
  page,
  onNavigate,
}: {
  items: NavItem[];
  page: PageId;
  onNavigate: (page: PageId) => void;
}) {
  return (
    <ul className="sidebar__list">
      {items.map((item) => (
        <li key={item.id}>
          <a
            href={`#/${item.id}`}
            className={`sidebar__item${
              page === item.id ? " sidebar__item--active" : ""
            }`}
            aria-current={page === item.id ? "page" : undefined}
            onClick={(event) => {
              event.preventDefault();
              if (item.id === "about") {
                useAboutStore.getState().openAbout();
              } else {
                onNavigate(item.id);
              }
            }}
          >
            {page === item.id && (
              <motion.span
                layoutId="sidebar-indicator"
                className="sidebar__indicator"
                transition={{ duration: 0.25, ease: [0.25, 1, 0.5, 1] }}
              />
            )}
            <Icon icon={item.icon} size={17} />
            <span>{item.label}</span>
          </a>
        </li>
      ))}
    </ul>
  );
}

export function Sidebar({ page, onNavigate }: SidebarProps) {
  const [query, setQuery] = useState("");

  const submitSearch = (value: string) => {
    if (value.trim()) {
      void useSearchStore.getState().search(value.trim());
      onNavigate("search");
    }
  };

  return (
    <nav className="sidebar" aria-label="主导航">
      <div className="sidebar__search">
        <MagnifyingGlass size={15} className="sidebar__search-icon" aria-hidden />
        <label htmlFor="sidebar-search" className="visually-hidden">
          搜索
        </label>
        <input
          id="sidebar-search"
          type="search"
          placeholder="搜索"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              submitSearch(query);
            }
          }}
        />
      </div>

      <div className="sidebar__group">
        <div className="sidebar__section-title">资料库</div>
        <NavList items={LIBRARY_ITEMS} page={page} onNavigate={onNavigate} />
      </div>

      <div className="sidebar__group">
        <div className="sidebar__section-title">管理</div>
        <NavList items={MANAGE_ITEMS} page={page} onNavigate={onNavigate} />
      </div>
    </nav>
  );
}
