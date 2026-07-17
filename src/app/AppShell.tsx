import {
  Activity,
  Boxes,
  FolderKanban,
  Gauge,
  HardDriveDownload,
  Network,
  PlugZap,
  Search,
  Settings,
  ShieldCheck,
  Wifi,
} from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { CommandPalette } from "../components/CommandPalette";
import { TaskCenter } from "../components/TaskCenter";
import { isDesktopRuntime } from "../lib/ipc";
import { useSettingsQuery } from "./queries";

const primaryNavigation = [
  { to: "/", label: "Overview", icon: Gauge },
  { to: "/processes", label: "Processes & Ports", icon: Activity },
  { to: "/projects", label: "Projects", icon: FolderKanban },
  { to: "/topology", label: "Topology", icon: Network },
  { to: "/docker", label: "Docker", icon: Boxes },
  { to: "/integrations", label: "Integrations", icon: PlugZap },
  { to: "/network", label: "Network", icon: Wifi },
  { to: "/system-diagnostics", label: "System Diagnostics", icon: Activity },
  { to: "/cleaner", label: "Cleaner", icon: HardDriveDownload },
  { to: "/permissions", label: "Data access", icon: ShieldCheck },
  { to: "/settings", label: "Settings", icon: Settings },
];

const plannedNavigation: { label: string; icon: typeof Boxes }[] = [];

export function AppShell() {
  const [paletteOpen, setPaletteOpen] = useState(false);
  const settings = useSettingsQuery();
  const desktopRuntime = isDesktopRuntime();
  const externalNetwork = settings.data?.externalNetworkChecks ?? false;

  useEffect(() => {
    document.documentElement.dataset.reducedMotion = settings.data?.reducedMotion
      ? "true"
      : "false";
  }, [settings.data?.reducedMotion]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="app-frame">
      <aside className="sidebar" aria-label="Application sidebar">
        <div className="brand-block">
          <div className="brand-mark" aria-hidden="true">
            MM
          </div>
          <div>
            <div className="brand-name">Mr Manager</div>
            <div className="brand-subtitle">Local command center</div>
          </div>
        </div>

        <nav className="nav-list" aria-label="Primary navigation">
          <div className="nav-section-label">Live</div>
          {primaryNavigation.map(({ to, label, icon: Icon }) => (
            <NavLink key={to} to={to} end={to === "/"} className="nav-item">
              <Icon size={17} aria-hidden="true" />
              <span>{label}</span>
            </NavLink>
          ))}

          {plannedNavigation.length > 0 && (
            <>
              <div className="nav-section-label nav-section-spaced">Next milestones</div>
              {plannedNavigation.map(({ label, icon: Icon }) => (
                <div key={label} className="nav-item nav-item-disabled" aria-disabled="true">
                  <Icon size={17} aria-hidden="true" />
                  <span>{label}</span>
                  <span className="planned-pill">Planned</span>
                </div>
              ))}
            </>
          )}
        </nav>

        <div className="sidebar-footer">
          <div className="privacy-state">
            <span className={`status-dot ${externalNetwork ? "status-dot-warning" : ""}`} />
            <div>
              <strong>{externalNetwork ? "External checks enabled" : "Local-only mode"}</strong>
              <span>{externalNetwork ? "Opt-in network access" : "No external requests"}</span>
            </div>
          </div>
        </div>
      </aside>

      <div className="workspace">
        <header className="topbar">
          <button className="command-trigger" type="button" onClick={() => setPaletteOpen(true)}>
            <Search size={15} aria-hidden="true" />
            <span>Search commands and views</span>
            <kbd>Ctrl K</kbd>
          </button>
          <TaskCenter />
          <div className="runtime-state" title="Desktop runtime state">
            <span className={`status-dot ${desktopRuntime ? "" : "status-dot-error"}`} />
            {desktopRuntime ? "Windows data connected" : "Web preview - system data unavailable"}
          </div>
        </header>
        <main className="main-content">
          <Outlet />
        </main>
      </div>

      <CommandPalette open={paletteOpen} onOpenChange={setPaletteOpen} />
    </div>
  );
}
