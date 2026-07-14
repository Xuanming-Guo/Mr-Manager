export type TopologyNodeKind = "project" | "commandRun" | "process" | "port" | "url";
export type TopologyEdgeKind =
  | "projectHasRun"
  | "runStartedProcess"
  | "projectContainsProcess"
  | "processParent"
  | "processOwnsPort"
  | "portExposesUrl";
export type TopologyConfidence = "certain" | "strong" | "inferred";

export interface TopologyEvidence {
  source: string;
  detail: string;
}

export interface TopologyMetadata {
  label: string;
  value: string;
}

export interface TopologyAction {
  id: string;
  label: string;
  url: string | null;
}

export interface TopologyNode {
  id: string;
  kind: TopologyNodeKind;
  label: string;
  detail: string | null;
  status: string | null;
  metadata: TopologyMetadata[];
  actions: TopologyAction[];
}

export interface TopologyEdge {
  id: string;
  from: string;
  to: string;
  kind: TopologyEdgeKind;
  confidence: TopologyConfidence;
  evidence: TopologyEvidence[];
}

export interface TopologyIssue {
  code: string;
  message: string;
}

export interface TopologyGraph {
  generatedAtMs: number;
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  issues: TopologyIssue[];
}
