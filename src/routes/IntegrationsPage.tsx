import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Bot, Boxes, Code2, Database, PlugZap, RefreshCw, Server, Shield } from "lucide-react";
import { useMemo, useState } from "react";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { AvailabilityBadge } from "../components/AvailabilityBadge";
import { integrationsApi } from "../lib/integrations-ipc";
import { formatBytes, formatTimestamp } from "../lib/format";
import type {
  IntegrationCategory,
  IntegrationInstalledState,
  IntegrationStatus,
  OllamaModel,
} from "../types/integrations";

const categoryLabels: Record<IntegrationCategory, string> = {
  runtime: "Runtimes",
  packageManager: "Package managers",
  editor: "Editors",
  container: "Containers",
  localAi: "Local AI",
  database: "Databases",
  shell: "Shells",
  vpn: "VPN evidence",
  localService: "Local services",
};

const categoryIcons: Record<IntegrationCategory, typeof PlugZap> = {
  runtime: Code2,
  packageManager: PlugZap,
  editor: Code2,
  container: Boxes,
  localAi: Bot,
  database: Database,
  shell: Server,
  vpn: Shield,
  localService: Server,
};

export function IntegrationsPage() {
  const client = useQueryClient();
  const [category, setCategory] = useState<IntegrationCategory | "all">("all");
  const integrations = useQuery({
    queryKey: ["integrations"],
    queryFn: integrationsApi.list,
    refetchInterval: 15_000,
  });
  const ollama = useQuery({
    queryKey: ["ollamaStatus"],
    queryFn: integrationsApi.ollama,
    refetchInterval: 15_000,
  });
  const wsl = useQuery({
    queryKey: ["wslStatus"],
    queryFn: integrationsApi.wsl,
    staleTime: 30_000,
  });

  const categories = useMemo(() => {
    const found = new Set((integrations.data ?? []).map((item) => item.category));
    return [...found].sort((left, right) =>
      categoryLabels[left].localeCompare(categoryLabels[right]),
    );
  }, [integrations.data]);

  const visible = useMemo(() => {
    const list = integrations.data ?? [];
    return category === "all" ? list : list.filter((item) => item.category === category);
  }, [category, integrations.data]);

  const error = integrations.error ?? ollama.error ?? wsl.error;

  if (integrations.isLoading)
    return <LoadingState label="Running bounded integration detectors..." />;
  if (integrations.error || !integrations.data) {
    return <ErrorState error={integrations.error} retry={() => void integrations.refetch()} />;
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <span className="eyebrow">Local services / Ollama / WSL</span>
          <h1>Integrations</h1>
        </div>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => {
            void integrations.refetch();
            void ollama.refetch();
            void wsl.refetch();
            void client.invalidateQueries({ queryKey: ["networkSnapshot"] });
          }}
          disabled={integrations.isFetching || ollama.isFetching || wsl.isFetching}
        >
          <RefreshCw
            size={15}
            className={integrations.isFetching || ollama.isFetching || wsl.isFetching ? "spin" : ""}
          />
          Refresh detectors
        </button>
      </header>

      {error && <ErrorState error={error} />}

      <section className="integration-hero-grid">
        <OllamaPanel status={ollama.data} loading={ollama.isLoading} />
        <WslPanel status={wsl.data} loading={wsl.isLoading} />
      </section>

      <section className="panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">Detector registry</span>
            <h2>Tools and local services</h2>
          </div>
          <PlugZap size={18} />
        </div>
        <div className="filter-row" role="tablist" aria-label="Integration categories">
          <button
            className={category === "all" ? "chip active" : "chip"}
            type="button"
            onClick={() => setCategory("all")}
          >
            All
          </button>
          {categories.map((item) => (
            <button
              key={item}
              className={category === item ? "chip active" : "chip"}
              type="button"
              onClick={() => setCategory(item)}
            >
              {categoryLabels[item]}
            </button>
          ))}
        </div>
        <div className="integration-grid">
          {visible.map((item) => (
            <IntegrationCard key={item.detectorId} integration={item} />
          ))}
        </div>
      </section>
    </div>
  );
}

function IntegrationCard({ integration }: { integration: IntegrationStatus }) {
  const Icon = categoryIcons[integration.category];
  return (
    <article className="integration-card">
      <div className="integration-card-header">
        <span className="integration-icon">
          <Icon size={18} />
        </span>
        <div>
          <h3>{integration.displayName}</h3>
          <small>{categoryLabels[integration.category]}</small>
        </div>
      </div>
      <div className="integration-status-row">
        <StatePill
          label={installedLabel(integration.installedState)}
          state={integration.installedState}
        />
        <span className={`state-pill state-${integration.runningState}`}>
          {integration.runningState}
        </span>
      </div>
      <dl className="compact-facts">
        <div>
          <dt>Version</dt>
          <dd>{integration.version ?? "Unavailable"}</dd>
        </div>
        <div>
          <dt>Processes</dt>
          <dd>{integration.processes.length}</dd>
        </div>
        <div>
          <dt>Endpoints</dt>
          <dd>{integration.endpoints.length}</dd>
        </div>
      </dl>
      {integration.endpoints.length > 0 && (
        <div className="mini-list">
          {integration.endpoints.slice(0, 3).map((endpoint) => (
            <span key={`${endpoint.port}-${endpoint.url ?? endpoint.evidence}`}>
              {endpoint.url ?? `port ${endpoint.port}`} ·{" "}
              {endpoint.localOnly ? "loopback" : "LAN candidate"}
            </span>
          ))}
        </div>
      )}
      <EvidenceList evidence={integration.evidence} />
      {integration.errors.length > 0 && (
        <div className="inline-warning">{integration.errors.slice(0, 2).join(" · ")}</div>
      )}
      <small className="muted">Checked {formatTimestamp(integration.lastCheckedAtMs)}</small>
    </article>
  );
}

function OllamaPanel({
  status,
  loading,
}: {
  status: Awaited<ReturnType<typeof integrationsApi.ollama>> | undefined;
  loading: boolean;
}) {
  if (loading) return <section className="panel">Checking Ollama loopback API...</section>;
  if (!status) return null;
  return (
    <section className="panel integration-feature-card">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Local AI</span>
          <h2>Ollama</h2>
        </div>
        <Bot size={18} />
      </div>
      <AvailabilityBadge state={status.availability.state} />
      <p className="panel-note">{status.availability.reason}</p>
      <dl className="compact-facts">
        <div>
          <dt>Endpoint</dt>
          <dd>{status.endpoint ?? "Not responding"}</dd>
        </div>
        <div>
          <dt>Version</dt>
          <dd>{status.version ?? "Unavailable"}</dd>
        </div>
        <div>
          <dt>Models</dt>
          <dd>
            {status.installedModels.length} installed · {status.runningModels.length} loaded
          </dd>
        </div>
      </dl>
      <ModelList title="Loaded models" models={status.runningModels} />
      <ModelList title="Installed models" models={status.installedModels.slice(0, 6)} />
      {status.errors.length > 0 && (
        <div className="inline-warning">{status.errors.join(" · ")}</div>
      )}
    </section>
  );
}

function WslPanel({
  status,
  loading,
}: {
  status: Awaited<ReturnType<typeof integrationsApi.wsl>> | undefined;
  loading: boolean;
}) {
  if (loading) return <section className="panel">Checking WSL distribution state...</section>;
  if (!status) return null;
  return (
    <section className="panel integration-feature-card">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">Read-only adapter</span>
          <h2>WSL</h2>
        </div>
        <Server size={18} />
      </div>
      <AvailabilityBadge state={status.availability.state} />
      <p className="panel-note">{status.availability.reason}</p>
      {status.distros.length === 0 ? (
        <div className="empty-inline">No WSL distributions were listed.</div>
      ) : (
        <div className="mini-table">
          {status.distros.map((distro) => (
            <div key={distro.name} className="mini-table-row">
              <strong>{distro.name}</strong>
              <span>{distro.state}</span>
              <span>WSL {distro.version ?? "?"}</span>
            </div>
          ))}
        </div>
      )}
      {status.errors.length > 0 && (
        <div className="inline-warning">{status.errors.join(" · ")}</div>
      )}
    </section>
  );
}

function ModelList({ title, models }: { title: string; models: OllamaModel[] }) {
  if (models.length === 0) return null;
  return (
    <div className="model-list">
      <strong>{title}</strong>
      {models.map((model) => (
        <span key={`${title}-${model.name}`}>
          {model.name}
          {model.parameterSize ? ` · ${model.parameterSize}` : ""}
          {model.sizeBytes ? ` · ${formatBytes(model.sizeBytes)}` : ""}
        </span>
      ))}
    </div>
  );
}

function EvidenceList({ evidence }: { evidence: IntegrationStatus["evidence"] }) {
  if (evidence.length === 0) {
    return <div className="mini-list muted">No evidence observed.</div>;
  }
  return (
    <div className="mini-list">
      {evidence.slice(0, 3).map((item) => (
        <span key={`${item.source}-${item.detail}`}>
          {item.source}: {item.detail} ({item.confidence})
        </span>
      ))}
    </div>
  );
}

function StatePill({ label, state }: { label: string; state: IntegrationInstalledState }) {
  return <span className={`state-pill state-${state}`}>{label}</span>;
}

function installedLabel(state: IntegrationInstalledState) {
  if (state === "notFound") return "not found";
  return state;
}
