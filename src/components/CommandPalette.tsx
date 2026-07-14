import * as Dialog from "@radix-ui/react-dialog";
import {
  Activity,
  Boxes,
  FolderGit2,
  Gauge,
  HardDriveDownload,
  Network,
  PlugZap,
  Search,
  Settings,
  ShieldCheck,
  Wifi,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const commands = [
  { label: "Open Overview", detail: "Live system summary", to: "/", icon: Gauge },
  {
    label: "Open Processes & Ports",
    detail: "Inspect ownership and resource use",
    to: "/processes",
    icon: Activity,
  },
  {
    label: "Open Projects",
    detail: "Registered roots, Git state, scripts, and setup health",
    to: "/projects",
    icon: FolderGit2,
  },
  {
    label: "Open Topology",
    detail: "Evidence graph, local URLs, QR, and preview",
    to: "/topology",
    icon: Network,
  },
  {
    label: "Open Docker",
    detail: "Containers, Compose files, logs, and doctor results",
    to: "/docker",
    icon: Boxes,
  },
  {
    label: "Open Integrations",
    detail: "Runtimes, local services, Ollama, WSL, and VPN evidence",
    to: "/integrations",
    icon: PlugZap,
  },
  {
    label: "Open Network",
    detail: "Local throughput, adapters, VPN evidence, and explicit internet diagnostics",
    to: "/network",
    icon: Wifi,
  },
  {
    label: "Open System Diagnostics",
    detail: "Live performance dashboard, recordings, timeline, and correlation",
    to: "/system-diagnostics",
    icon: Activity,
  },
  {
    label: "Open Cleaner",
    detail: "Review build artifacts, quarantine safely, restore, or confirm purge",
    to: "/cleaner",
    icon: HardDriveDownload,
  },
  {
    label: "Open Permissions",
    detail: "Review supported and read-only capabilities",
    to: "/permissions",
    icon: ShieldCheck,
  },
  {
    label: "Open Settings",
    detail: "Refresh, privacy, and accessibility",
    to: "/settings",
    icon: Settings,
  },
];

export function CommandPalette({ open, onOpenChange }: CommandPaletteProps) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return needle
      ? commands.filter((command) =>
          `${command.label} ${command.detail}`.toLowerCase().includes(needle),
        )
      : commands;
  }, [query]);

  const run = (to: string) => {
    navigate(to);
    onOpenChange(false);
    setQuery("");
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="command-dialog" aria-describedby={undefined}>
          <Dialog.Title className="sr-only">Command palette</Dialog.Title>
          <div className="command-search-row">
            <Search size={18} aria-hidden="true" />
            <input
              autoFocus
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search Mr Manager..."
              aria-label="Search commands"
            />
            <Dialog.Close className="icon-button" aria-label="Close command palette">
              <X size={16} />
            </Dialog.Close>
          </div>
          <div className="command-results" role="listbox" aria-label="Commands">
            {filtered.map(({ label, detail, to, icon: Icon }) => (
              <button
                key={to}
                type="button"
                className="command-result"
                onClick={() => run(to)}
                role="option"
                aria-selected="false"
              >
                <span className="command-result-icon">
                  <Icon size={17} />
                </span>
                <span>
                  <strong>{label}</strong>
                  <small>{detail}</small>
                </span>
              </button>
            ))}
            {filtered.length === 0 && <div className="empty-command">No matching command.</div>}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
