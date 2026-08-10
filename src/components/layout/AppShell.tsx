import type { ReactNode } from "react";
import { Sidebar, type PageId } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { MiniPlayer } from "../player/MiniPlayer";
import { FullPlayer } from "../player/FullPlayer";
import { QueuePanel } from "../player/QueuePanel";

interface AppShellProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
  children: ReactNode;
}

export function AppShell({ page, onNavigate, children }: AppShellProps) {
  return (
    <div className="app-shell">
      <TitleBar />
      <div className="app-shell__body">
        <Sidebar page={page} onNavigate={onNavigate} />
        <main className="app-shell__content" id="main-content">
          {children}
        </main>
      </div>
      <MiniPlayer />
      <FullPlayer />
      <QueuePanel />
    </div>
  );
}
