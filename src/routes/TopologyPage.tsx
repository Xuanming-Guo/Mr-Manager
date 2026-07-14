import {
  Background,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useMutation, useQuery } from "@tanstack/react-query";
import { ExternalLink, GitBranch, Network, QrCode, RefreshCw } from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import QRCode from "react-qr-code";
import { ErrorState, LoadingState } from "../components/AsyncState";
import { topologyApi } from "../lib/topology-ipc";
import type { TopologyEdge, TopologyGraph, TopologyNode } from "../types/topology";

const nodeColumns: Record<TopologyNode["kind"], number> = {
  project: 0,
  commandRun: 280,
  process: 560,
  port: 840,
  url: 1120,
};

type GraphView = "live" | "projects" | "all";

export function TopologyPage() {
  const graph = useQuery({
    queryKey: ["topologyGraph"],
    queryFn: topologyApi.graph,
    refetchInterval: 2_000,
  });
  const preview = useMutation({ mutationFn: topologyApi.openPreview });

  if (graph.isLoading) return <LoadingState label="Building evidence graph..." />;
  if (graph.error) return <ErrorState error={graph.error} retry={() => void graph.refetch()} />;

  return (
    <TopologyCanvas
      graph={graph.data}
      refreshing={graph.isFetching}
      previewing={preview.isPending}
      previewError={preview.error}
      onRefresh={() => void graph.refetch()}
      onPreview={(url) => preview.mutate(url)}
    />
  );
}

function TopologyCanvas({
  graph,
  refreshing,
  previewing,
  previewError,
  onRefresh,
  onPreview,
}: {
  graph: TopologyGraph | undefined;
  refreshing: boolean;
  previewing: boolean;
  previewError: unknown;
  onRefresh: () => void;
  onPreview: (url: string) => void;
}) {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [view, setView] = useState<GraphView>("live");
  const raw = graph ?? { generatedAtMs: 0, nodes: [], edges: [], issues: [] };
  const data = filterGraph(raw, view);
  const flowNodes = toFlowNodes(data.nodes);
  const flowEdges = toFlowEdges(data.edges);
  const selectedNode =
    data.nodes.find((node) => node.id === selectedNodeId) ??
    data.nodes.find((node) => node.kind === "url") ??
    data.nodes[0] ??
    null;
  const selectedUrl = selectedNode?.kind === "url" ? selectedNode.label : null;
  const selectedEdges = selectedNode
    ? data.edges.filter((edge) => edge.from === selectedNode.id || edge.to === selectedNode.id)
    : data.edges.slice(0, 4);

  return (
    <div className="page-stack page-fill">
      <header className="page-header">
        <div>
          <span className="eyebrow">Evidence graph</span>
          <h1>Topology</h1>
        </div>
        <button className="button button-secondary" type="button" onClick={onRefresh}>
          <RefreshCw size={15} className={refreshing ? "spin" : ""} />
          Refresh
        </button>
      </header>

      {previewError !== null && previewError !== undefined && <ErrorState error={previewError} />}
      {data.issues.length > 0 && (
        <div className="topology-issues">
          {data.issues.map((issue) => (
            <div key={issue.code}>
              <GitBranch size={14} />
              <span>
                <strong>{issue.code}</strong>
                {issue.message}
              </span>
            </div>
          ))}
        </div>
      )}

      <section className="topology-toolbar" aria-label="Topology view filters">
        <div className="segmented-control">
          {(["live", "projects", "all"] as const).map((option) => (
            <button
              key={option}
              type="button"
              className={view === option ? "selected" : ""}
              onClick={() => setView(option)}
            >
              {option === "live"
                ? "Live services"
                : option === "projects"
                  ? "Project map"
                  : "All evidence"}
            </button>
          ))}
        </div>
        <div className="topology-counts">
          <span>{data.nodes.filter((node) => node.kind === "process").length} processes</span>
          <span>{data.nodes.filter((node) => node.kind === "port").length} ports</span>
          <span>{data.nodes.filter((node) => node.kind === "url").length} URLs</span>
        </div>
        <div className="confidence-legend">
          <span className="confidence-certain">Certain</span>
          <span className="confidence-strong">Strong</span>
          <span className="confidence-inferred">Inferred</span>
          <Link to="/permissions">What does evidence mean?</Link>
        </div>
      </section>

      <div className="topology-layout">
        <section className="topology-canvas" aria-label="Evidence topology graph">
          {flowNodes.length ? (
            <ReactFlow
              nodes={flowNodes}
              edges={flowEdges}
              fitView
              minZoom={0.35}
              maxZoom={1.4}
              onNodeClick={(_, node) => setSelectedNodeId(node.id)}
            >
              <MiniMap pannable zoomable />
              <Controls />
              <Background />
            </ReactFlow>
          ) : (
            <div className="project-empty">
              <Network size={30} />
              <h2>No graph evidence yet</h2>
              <p>Register a project and start a managed command to create live graph edges.</p>
            </div>
          )}
        </section>

        <aside className="topology-panel">
          <div className="topology-panel-card">
            <span className="eyebrow">Selected node</span>
            {selectedNode ? (
              <>
                <h2>{selectedNode.label}</h2>
                <p>{selectedNode.detail ?? selectedNode.kind}</p>
                <div className="metadata-list">
                  <div>
                    <span>Kind</span>
                    <strong>{selectedNode.kind}</strong>
                  </div>
                  {selectedNode.status && (
                    <div>
                      <span>Status</span>
                      <strong>{selectedNode.status}</strong>
                    </div>
                  )}
                  {selectedNode.metadata.map((item) => (
                    <div key={`${item.label}:${item.value}`}>
                      <span>{item.label}</span>
                      <strong>{item.value}</strong>
                    </div>
                  ))}
                </div>
                {selectedUrl && (
                  <div className="qr-card">
                    <QrCode size={15} />
                    <QRCode value={selectedUrl} size={118} />
                    <button
                      type="button"
                      className="button button-primary"
                      onClick={() => onPreview(selectedUrl)}
                      disabled={previewing}
                    >
                      <ExternalLink size={14} />
                      Preview
                    </button>
                  </div>
                )}
              </>
            ) : (
              <div className="empty-inline">No topology nodes are available.</div>
            )}
          </div>

          <div className="topology-panel-card">
            <span className="eyebrow">Evidence</span>
            {selectedEdges.length ? (
              <div className="evidence-list">
                {selectedEdges.map((edge) => (
                  <div key={edge.id}>
                    <strong>
                      {edge.kind} - {edge.confidence}
                    </strong>
                    {edge.evidence.map((item) => (
                      <p key={`${edge.id}:${item.source}:${item.detail}`}>
                        <span>{item.source}</span>
                        {item.detail}
                      </p>
                    ))}
                  </div>
                ))}
              </div>
            ) : (
              <div className="empty-inline">
                No relationship is proven for this node yet. It remains visible rather than being
                guessed into a project.
              </div>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}

function toFlowNodes(nodes: TopologyNode[]): Node[] {
  const rowByKind = new Map<TopologyNode["kind"], number>();
  return nodes.map((node) => {
    const row = rowByKind.get(node.kind) ?? 0;
    rowByKind.set(node.kind, row + 1);
    return {
      id: node.id,
      position: { x: nodeColumns[node.kind], y: row * 118 },
      data: { label: nodeLabel(node) },
      className: `topology-flow-node topology-flow-node-${node.kind}`,
    };
  });
}

function toFlowEdges(edges: TopologyEdge[]): Edge[] {
  return edges.map((edge) => ({
    id: edge.id,
    source: edge.from,
    target: edge.to,
    label: `${edgeLabel(edge.kind)} · ${edge.confidence}`,
    animated: edge.confidence === "inferred",
    markerEnd: { type: MarkerType.ArrowClosed },
    className: `topology-flow-edge topology-flow-edge-${edge.confidence}`,
  }));
}

function nodeLabel(node: TopologyNode) {
  return (
    <div className="topology-node-label">
      <span>{nodeKindLabel(node.kind)}</span>
      <strong>{node.label}</strong>
      {node.status && <small>{node.status}</small>}
    </div>
  );
}

function nodeKindLabel(kind: TopologyNode["kind"]) {
  const labels: Record<TopologyNode["kind"], string> = {
    project: "Project",
    commandRun: "Managed run",
    process: "Process",
    port: "Listening port",
    url: "Local URL",
  };
  return labels[kind];
}

function edgeLabel(kind: TopologyEdge["kind"]) {
  const labels: Record<TopologyEdge["kind"], string> = {
    projectHasRun: "runs",
    runStartedProcess: "started",
    projectContainsProcess: "associated with",
    processParent: "parent of",
    processOwnsPort: "listens on",
    portExposesUrl: "opens as",
  };
  return labels[kind];
}

function filterGraph(graph: TopologyGraph, view: GraphView): TopologyGraph {
  if (view === "all") return graph;
  const nodeById = new Map(graph.nodes.map((node) => [node.id, node]));
  const included = new Set<string>();
  if (view === "live") {
    for (const node of graph.nodes) {
      if (node.kind === "port" || node.kind === "url") {
        included.add(node.id);
      }
    }
  } else {
    for (const node of graph.nodes) {
      if (node.kind === "project") included.add(node.id);
    }
  }
  let changed = true;
  while (changed) {
    changed = false;
    for (const edge of graph.edges) {
      const fromIncluded = included.has(edge.from);
      const toIncluded = included.has(edge.to);
      if (view === "live" && (fromIncluded || toIncluded)) {
        const other = fromIncluded ? edge.to : edge.from;
        const otherNode = nodeById.get(other);
        if (
          otherNode &&
          !included.has(other) &&
          (otherNode.kind === "project" ||
            otherNode.kind === "commandRun" ||
            otherNode.kind === "process")
        ) {
          included.add(other);
          changed = true;
        }
      }
      if (view === "projects" && (fromIncluded || toIncluded)) {
        const other = fromIncluded ? edge.to : edge.from;
        if (!included.has(other)) {
          included.add(other);
          changed = true;
        }
      }
    }
  }
  return {
    ...graph,
    nodes: graph.nodes.filter((node) => included.has(node.id)),
    edges: graph.edges.filter((edge) => included.has(edge.from) && included.has(edge.to)),
  };
}
