import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import type { PortEndpoint } from "../types/system";

type ScopeFilter = "all" | "localhost" | "lan";
type ProtocolFilter = "all" | "tcp" | "udp";
type ServiceFilter = "all" | "common" | "web" | "data" | "other";
type OwnerFilter = "all" | "known" | "unknown";

const webPorts = new Set([80, 443, 3000, 3001, 4173, 4200, 5000, 5173, 5174, 8000, 8080, 8787]);
const dataPorts = new Set([1433, 3306, 5432, 6379, 9200, 27017]);

export function PortTable({ endpoints }: { endpoints: PortEndpoint[] }) {
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<ScopeFilter>("all");
  const [protocol, setProtocol] = useState<ProtocolFilter>("all");
  const [service, setService] = useState<ServiceFilter>("all");
  const [owner, setOwner] = useState<OwnerFilter>("all");
  const rows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return endpoints
      .filter(
        (endpoint) =>
          scope === "all" ||
          (scope === "localhost"
            ? endpoint.bindingScope === "loopback"
            : endpoint.bindingScope !== "loopback"),
      )
      .filter((endpoint) => protocol === "all" || endpoint.protocol === protocol)
      .filter(
        (endpoint) =>
          owner === "all" ||
          (owner === "known"
            ? endpoint.owningProcessName !== null
            : endpoint.owningProcessName === null),
      )
      .filter((endpoint) => serviceMatches(endpoint.localPort, service))
      .filter(
        (endpoint) =>
          !needle ||
          `${endpoint.localPort} ${endpoint.localAddress} ${endpoint.owningProcessName ?? ""}`
            .toLowerCase()
            .includes(needle),
      )
      .sort((a, b) => a.localPort - b.localPort);
  }, [endpoints, owner, protocol, query, scope, service]);

  return (
    <div className="data-table-shell">
      <div className="table-toolbar">
        <label className="search-field">
          <Search size={15} />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search port, address, or owner"
          />
        </label>
        <label className="select-field">
          Binding
          <select value={scope} onChange={(event) => setScope(event.target.value as ScopeFilter)}>
            <option value="all">All bindings</option>
            <option value="localhost">Localhost only</option>
            <option value="lan">LAN-visible</option>
          </select>
        </label>
        <label className="select-field">
          Protocol
          <select
            value={protocol}
            onChange={(event) => setProtocol(event.target.value as ProtocolFilter)}
          >
            <option value="all">TCP + UDP</option>
            <option value="tcp">TCP</option>
            <option value="udp">UDP</option>
          </select>
        </label>
        <label className="select-field">
          Service
          <select
            value={service}
            onChange={(event) => setService(event.target.value as ServiceFilter)}
          >
            <option value="all">All ports</option>
            <option value="common">Common development</option>
            <option value="web">Web / app servers</option>
            <option value="data">Database / cache</option>
            <option value="other">Other ports</option>
          </select>
        </label>
        <label className="select-field">
          Owner
          <select value={owner} onChange={(event) => setOwner(event.target.value as OwnerFilter)}>
            <option value="all">Any owner</option>
            <option value="known">Known owner</option>
            <option value="unknown">Unknown owner</option>
          </select>
        </label>
        <span className="row-count">{rows.length} endpoints</span>
      </div>
      <div className="port-grid port-grid-header" role="row">
        <span>Protocol</span>
        <span>Address</span>
        <span>Port</span>
        <span>Owner</span>
        <span>Binding</span>
        <span>URL</span>
      </div>
      <div className="port-rows" role="table">
        {rows.map((endpoint) => (
          <div
            className="port-grid port-grid-row"
            role="row"
            key={`${endpoint.protocol}:${endpoint.localAddress}:${endpoint.localPort}:${endpoint.owningProcessKey?.pid ?? 0}`}
          >
            <span className={`protocol-pill protocol-${endpoint.protocol}`}>
              {endpoint.protocol.toUpperCase()}
            </span>
            <code>{endpoint.localAddress}</code>
            <strong>{endpoint.localPort}</strong>
            <span>
              {endpoint.owningProcessName
                ? `${endpoint.owningProcessName} · ${endpoint.owningProcessKey?.pid ?? "?"}`
                : "Unavailable"}
            </span>
            <span className="binding-label">
              {endpoint.bindingScope === "allInterfaces"
                ? "All interfaces"
                : endpoint.bindingScope === "loopback"
                  ? "Loopback only"
                  : "Specific interface"}
            </span>
            <span>{endpoint.localUrl ? <code>{endpoint.localUrl}</code> : "—"}</span>
          </div>
        ))}
        {rows.length === 0 && (
          <div className="empty-state">No endpoints match the current filters.</div>
        )}
      </div>
    </div>
  );
}

function serviceMatches(port: number, filter: ServiceFilter) {
  if (filter === "all") return true;
  const web = webPorts.has(port);
  const data = dataPorts.has(port);
  if (filter === "common") return web || data;
  if (filter === "web") return web;
  if (filter === "data") return data;
  return !web && !data;
}
