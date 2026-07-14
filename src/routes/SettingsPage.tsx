import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Gauge, Globe2, Save, Sparkles } from "lucide-react";
import { useState } from "react";
import { useSettingsQuery } from "../app/queries";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { desktopApi } from "../lib/ipc";
import type { AppSettings, RefreshMode } from "../types/system";

const modes: Array<{ id: RefreshMode; label: string; detail: string; icon: typeof Gauge }> = [
  {
    id: "normal",
    label: "Normal",
    detail: "2.5 second updates for everyday use",
    icon: Gauge,
  },
  {
    id: "fast",
    label: "Fast",
    detail: "1 second updates with higher overhead",
    icon: Sparkles,
  },
];

export function SettingsPage() {
  const current = useSettingsQuery();
  const client = useQueryClient();
  const [overrides, setOverrides] = useState<Partial<AppSettings>>({});
  const save = useMutation({
    mutationFn: desktopApi.updateSettings,
    onSuccess: (settings) => {
      client.setQueryData(["settings"], settings);
      setOverrides({});
    },
  });

  if (current.isLoading || !current.data) return <LoadingState label="Loading local settings…" />;
  if (current.error)
    return <ErrorState error={current.error} retry={() => void current.refetch()} />;

  const draft: AppSettings = { ...current.data, ...overrides };
  const toggle = (key: "externalNetworkChecks" | "metricHistoryEnabled" | "reducedMotion") =>
    setOverrides((value) => ({ ...value, [key]: !draft[key] }));

  return (
    <div className="page-stack settings-page">
      <header className="page-header">
        <div>
          <span className="eyebrow">Local preferences</span>
          <h1>Settings</h1>
        </div>
        <button
          type="button"
          className="button button-primary"
          onClick={() => save.mutate(draft)}
          disabled={save.isPending}
        >
          <Save size={15} />
          {save.isPending ? "Saving…" : "Save locally"}
        </button>
      </header>
      {save.error && <ErrorState error={save.error} />}
      {save.isSuccess && (
        <div className="success-banner" role="status">
          Settings saved to the local Mr Manager database.
        </div>
      )}

      <section className="panel settings-section">
        <div className="settings-heading">
          <Gauge size={19} />
          <div>
            <h2>Refresh mode</h2>
            <p>Controls coordinated system, process, and port collection.</p>
          </div>
        </div>
        <div className="mode-grid">
          {modes.map(({ id, label, detail, icon: Icon }) => (
            <button
              type="button"
              key={id}
              className={`mode-card ${draft.refreshMode === id ? "mode-card-selected" : ""}`}
              onClick={() => setOverrides((value) => ({ ...value, refreshMode: id }))}
            >
              <Icon size={18} />
              <strong>{label}</strong>
              <span>{detail}</span>
              <span className="radio-indicator" />
            </button>
          ))}
        </div>
      </section>

      <section className="panel settings-section">
        <div className="settings-heading">
          <Globe2 size={19} />
          <div>
            <h2>Privacy & retention</h2>
            <p>These controls never enable themselves automatically.</p>
          </div>
        </div>
        <div className="toggle-list">
          <button
            type="button"
            className="toggle-row"
            onClick={() => toggle("externalNetworkChecks")}
          >
            <span>
              <strong>External network checks</strong>
              <small>
                Allow explicitly initiated registry, public-IP, and dependency checks later.
              </small>
            </span>
            <span
              className={`toggle ${draft.externalNetworkChecks ? "toggle-on" : ""}`}
              aria-label={draft.externalNetworkChecks ? "Enabled" : "Disabled"}
            >
              <span />
            </span>
          </button>
          <button
            type="button"
            className="toggle-row"
            onClick={() => toggle("metricHistoryEnabled")}
          >
            <span>
              <strong>Metric history</strong>
              <small>Persist history only when enabled; live data otherwise stays in memory.</small>
            </span>
            <span className={`toggle ${draft.metricHistoryEnabled ? "toggle-on" : ""}`}>
              <span />
            </span>
          </button>
          <button type="button" className="toggle-row" onClick={() => toggle("reducedMotion")}>
            <span>
              <strong>Reduced motion</strong>
              <small>Disable non-essential transitions and lifecycle animation.</small>
            </span>
            <span className={`toggle ${draft.reducedMotion ? "toggle-on" : ""}`}>
              <span />
            </span>
          </button>
        </div>
      </section>
    </div>
  );
}
