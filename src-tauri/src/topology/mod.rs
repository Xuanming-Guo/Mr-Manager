use std::collections::{BTreeSet, HashMap};

use crate::collector::CollectedSnapshot;
use crate::domain::{
    BindingScope, ManagedCommand, PortEndpoint, ProcessSnapshot, Project, TopologyAction,
    TopologyConfidence, TopologyEdge, TopologyEdgeKind, TopologyEvidence, TopologyGraph,
    TopologyIssue, TopologyMetadata, TopologyNode, TopologyNodeKind,
};
use crate::projects::now_ms;

pub fn build_topology(
    snapshot: CollectedSnapshot,
    projects: Vec<Project>,
    runs: Vec<ManagedCommand>,
) -> TopologyGraph {
    let process_by_pid = snapshot
        .processes
        .iter()
        .map(|process| (process.key.pid, process))
        .collect::<HashMap<_, _>>();
    let mut builder = TopologyBuilder::new();
    let mut included_processes = BTreeSet::<u32>::new();
    let mut project_associated_processes = BTreeSet::<u32>::new();

    for project in &projects {
        builder.add_node(project_node(project));
    }

    for run in &runs {
        builder.add_node(run_node(run));
        builder.add_edge(
            project_node_id(&run.project_id),
            run_node_id(&run.run_id),
            TopologyEdgeKind::ProjectHasRun,
            TopologyConfidence::Certain,
            vec![TopologyEvidence {
                source: "managed-supervisor".to_owned(),
                detail: "Mr Manager launched this command for the registered project.".to_owned(),
            }],
        );

        if let Some(pid) = run.pid
            && let Some(process) = process_by_pid.get(&pid)
        {
            builder.add_node(process_node(process));
            included_processes.insert(pid);
            project_associated_processes.insert(pid);
            builder.add_edge(
                run_node_id(&run.run_id),
                process_node_id(process),
                TopologyEdgeKind::RunStartedProcess,
                TopologyConfidence::Certain,
                vec![TopologyEvidence {
                    source: "managed-supervisor".to_owned(),
                    detail: format!("The supervisor recorded PID {pid} for this run."),
                }],
            );
            builder.add_edge(
                project_node_id(&run.project_id),
                process_node_id(process),
                TopologyEdgeKind::ProjectContainsProcess,
                TopologyConfidence::Certain,
                vec![TopologyEvidence {
                    source: "managed-supervisor".to_owned(),
                    detail: "The process was launched from a Mr Manager project command."
                        .to_owned(),
                }],
            );
        }
    }

    for project in &projects {
        for process in &snapshot.processes {
            if process
                .cwd
                .as_deref()
                .is_some_and(|cwd| path_inside(cwd, &project.canonical_root_path))
            {
                builder.add_node(process_node(process));
                included_processes.insert(process.key.pid);
                project_associated_processes.insert(process.key.pid);
                builder.add_edge(
                    project_node_id(&project.id),
                    process_node_id(process),
                    TopologyEdgeKind::ProjectContainsProcess,
                    TopologyConfidence::Strong,
                    vec![TopologyEvidence {
                        source: "process-cwd".to_owned(),
                        detail: "The process working directory lies inside the project root."
                            .to_owned(),
                    }],
                );
            }

            if process
                .command_line_redacted
                .as_deref()
                .is_some_and(|command| {
                    path_text_mentions_root(command, &project.canonical_root_path)
                })
            {
                builder.add_node(process_node(process));
                included_processes.insert(process.key.pid);
                project_associated_processes.insert(process.key.pid);
                builder.add_edge(
                    project_node_id(&project.id),
                    process_node_id(process),
                    TopologyEdgeKind::ProjectContainsProcess,
                    TopologyConfidence::Strong,
                    vec![TopologyEvidence {
                        source: "process-command-line".to_owned(),
                        detail: "The redacted process command line references the registered project root."
                            .to_owned(),
                    }],
                );
            }
        }
    }

    let mut live_service_processes = BTreeSet::new();
    for port in &snapshot.ports {
        if port.local_url.is_none() && port.lan_urls.is_empty() {
            continue;
        }
        let Some(process_key) = port.owning_process_key else {
            continue;
        };
        let Some(process) = process_by_pid.get(&process_key.pid) else {
            continue;
        };
        builder.add_node(process_node(process));
        included_processes.insert(process_key.pid);
        live_service_processes.insert(process_key.pid);
    }

    add_parent_edges(&mut builder, &process_by_pid, &mut included_processes);
    add_port_edges(
        &mut builder,
        &snapshot.ports,
        &process_by_pid,
        &included_processes,
    );

    let unassociated_live_services = live_service_processes
        .difference(&project_associated_processes)
        .count();
    if unassociated_live_services > 0 {
        builder.issues.push(TopologyIssue {
            code: "UNASSOCIATED_LIVE_SERVICES".to_owned(),
            message: format!(
                "{unassociated_live_services} live development service process(es) are visible without enough evidence for a project association."
            ),
        });
    }

    if builder
        .edges
        .iter()
        .any(|edge| edge.kind == TopologyEdgeKind::PortExposesUrl)
        && snapshot.ports.iter().any(|port| {
            port.binding_scope == BindingScope::AllInterfaces && port.lan_urls.is_empty()
        })
    {
        builder.issues.push(TopologyIssue {
            code: "LAN_ADAPTER_CANDIDATES_PENDING".to_owned(),
            message: "All-interface bindings are visible, but concrete LAN address candidates require the network-adapter milestone.".to_owned(),
        });
    }

    builder.finish()
}

fn add_parent_edges(
    builder: &mut TopologyBuilder,
    process_by_pid: &HashMap<u32, &ProcessSnapshot>,
    included_processes: &mut BTreeSet<u32>,
) {
    let children = included_processes.iter().copied().collect::<Vec<_>>();
    for child_pid in children {
        let Some(child) = process_by_pid.get(&child_pid) else {
            continue;
        };
        let Some(parent_pid) = child.parent_pid else {
            continue;
        };
        let Some(parent) = process_by_pid.get(&parent_pid) else {
            continue;
        };
        builder.add_node(process_node(parent));
        included_processes.insert(parent_pid);
        builder.add_edge(
            process_node_id(parent),
            process_node_id(child),
            TopologyEdgeKind::ProcessParent,
            TopologyConfidence::Certain,
            vec![TopologyEvidence {
                source: "process-snapshot".to_owned(),
                detail: format!("Windows process snapshot reports parent PID {parent_pid}."),
            }],
        );
    }
}

fn add_port_edges(
    builder: &mut TopologyBuilder,
    ports: &[PortEndpoint],
    process_by_pid: &HashMap<u32, &ProcessSnapshot>,
    included_processes: &BTreeSet<u32>,
) {
    for port in ports {
        let Some(process_key) = port.owning_process_key else {
            continue;
        };
        if !included_processes.contains(&process_key.pid) {
            continue;
        }
        let Some(process) = process_by_pid.get(&process_key.pid) else {
            continue;
        };
        let port_id = port_node_id(port);
        builder.add_node(port_node(port));
        builder.add_edge(
            process_node_id(process),
            port_id.clone(),
            TopologyEdgeKind::ProcessOwnsPort,
            TopologyConfidence::Certain,
            port.evidence
                .iter()
                .map(|detail| TopologyEvidence {
                    source: "windows-port-table".to_owned(),
                    detail: detail.clone(),
                })
                .collect(),
        );

        for url in port
            .local_url
            .iter()
            .chain(port.lan_urls.iter())
            .filter(|url| !url.is_empty())
        {
            builder.add_node(url_node(url, port));
            builder.add_edge(
                port_id.clone(),
                url_node_id(url),
                TopologyEdgeKind::PortExposesUrl,
                TopologyConfidence::Inferred,
                vec![TopologyEvidence {
                    source: "port-url-inference".to_owned(),
                    detail: "The URL is derived from the binding address, port, and common development-port scheme inference.".to_owned(),
                }],
            );
        }
    }
}

fn project_node(project: &Project) -> TopologyNode {
    TopologyNode {
        id: project_node_id(&project.id),
        kind: TopologyNodeKind::Project,
        label: project.name.clone(),
        detail: Some(project.root_path.clone()),
        status: Some(format!("{:?}", project.scan_health.state).to_lowercase()),
        metadata: vec![
            TopologyMetadata {
                label: "Stacks".to_owned(),
                value: if project.detected_stacks.is_empty() {
                    "none detected".to_owned()
                } else {
                    project
                        .detected_stacks
                        .iter()
                        .map(|stack| format!("{stack:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            },
            TopologyMetadata {
                label: "Scripts".to_owned(),
                value: project.scripts.len().to_string(),
            },
        ],
        actions: Vec::new(),
    }
}

fn run_node(run: &ManagedCommand) -> TopologyNode {
    TopologyNode {
        id: run_node_id(&run.run_id),
        kind: TopologyNodeKind::CommandRun,
        label: run.label.clone(),
        detail: Some(format!("{} {}", run.executable, run.arguments.join(" "))),
        status: Some(format!("{:?}", run.state).to_lowercase()),
        metadata: vec![
            TopologyMetadata {
                label: "PID".to_owned(),
                value: run
                    .pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "not started".to_owned()),
            },
            TopologyMetadata {
                label: "Logs".to_owned(),
                value: run.log_count.to_string(),
            },
        ],
        actions: Vec::new(),
    }
}

fn process_node(process: &ProcessSnapshot) -> TopologyNode {
    TopologyNode {
        id: process_node_id(process),
        kind: TopologyNodeKind::Process,
        label: process.name.clone(),
        detail: process
            .cwd
            .clone()
            .or_else(|| process.executable_path.clone()),
        status: Some(process.status.clone()),
        metadata: vec![
            TopologyMetadata {
                label: "PID".to_owned(),
                value: process.key.pid.to_string(),
            },
            TopologyMetadata {
                label: "Listening ports".to_owned(),
                value: process.listening_port_count.to_string(),
            },
        ],
        actions: Vec::new(),
    }
}

fn port_node(port: &PortEndpoint) -> TopologyNode {
    TopologyNode {
        id: port_node_id(port),
        kind: TopologyNodeKind::Port,
        label: format!(
            "{} {}:{}",
            format!("{:?}", port.protocol).to_uppercase(),
            port.local_address,
            port.local_port
        ),
        detail: port.owning_process_name.clone(),
        status: Some(format!("{:?}", port.binding_scope).to_lowercase()),
        metadata: vec![
            TopologyMetadata {
                label: "State".to_owned(),
                value: port.state.clone(),
            },
            TopologyMetadata {
                label: "Reachability".to_owned(),
                value: format!("{:?}", port.reachability_state).to_lowercase(),
            },
        ],
        actions: Vec::new(),
    }
}

fn url_node(url: &str, port: &PortEndpoint) -> TopologyNode {
    TopologyNode {
        id: url_node_id(url),
        kind: TopologyNodeKind::Url,
        label: url.to_owned(),
        detail: Some(binding_explanation(port)),
        status: port.inferred_scheme.clone(),
        metadata: vec![TopologyMetadata {
            label: "Binding".to_owned(),
            value: format!("{:?}", port.binding_scope).to_lowercase(),
        }],
        actions: vec![TopologyAction {
            id: "openPreview".to_owned(),
            label: "Open isolated preview".to_owned(),
            url: Some(url.to_owned()),
        }],
    }
}

fn binding_explanation(port: &PortEndpoint) -> String {
    match port.binding_scope {
        BindingScope::Loopback => "Loopback binding; not exposed to the LAN by this binding."
            .to_owned(),
        BindingScope::AllInterfaces => "All-interface binding; other device reachability is not claimed without a companion check."
            .to_owned(),
        BindingScope::SpecificInterface => "Specific-interface binding; reachability depends on adapter, firewall, and network policy."
            .to_owned(),
    }
}

fn project_node_id(project_id: &str) -> String {
    format!("project:{project_id}")
}

fn run_node_id(run_id: &str) -> String {
    format!("run:{run_id}")
}

fn process_node_id(process: &ProcessSnapshot) -> String {
    format!("process:{}:{}", process.key.pid, process.key.start_time)
}

fn port_node_id(port: &PortEndpoint) -> String {
    format!(
        "port:{:?}:{}:{}",
        port.protocol, port.local_address, port.local_port
    )
}

fn url_node_id(url: &str) -> String {
    format!("url:{url}")
}

fn path_inside(candidate: &str, root: &str) -> bool {
    let candidate = normalize_path(candidate);
    let root = normalize_path(root);
    candidate == root || candidate.starts_with(&format!("{root}\\"))
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn path_text_mentions_root(text: &str, root: &str) -> bool {
    let text = text.replace('/', "\\").to_ascii_lowercase();
    let root = normalize_path(root);
    !root.is_empty() && text.contains(&root)
}

struct TopologyBuilder {
    nodes: Vec<TopologyNode>,
    edges: Vec<TopologyEdge>,
    issues: Vec<TopologyIssue>,
    node_ids: BTreeSet<String>,
    edge_ids: BTreeSet<String>,
}

impl TopologyBuilder {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            issues: Vec::new(),
            node_ids: BTreeSet::new(),
            edge_ids: BTreeSet::new(),
        }
    }

    fn add_node(&mut self, node: TopologyNode) {
        if self.node_ids.insert(node.id.clone()) {
            self.nodes.push(node);
        }
    }

    fn add_edge(
        &mut self,
        from: String,
        to: String,
        kind: TopologyEdgeKind,
        confidence: TopologyConfidence,
        evidence: Vec<TopologyEvidence>,
    ) {
        if !self.node_ids.contains(&from) || !self.node_ids.contains(&to) || evidence.is_empty() {
            return;
        }
        let id = format!("{kind:?}:{from}->{to}");
        if self.edge_ids.insert(id.clone()) {
            self.edges.push(TopologyEdge {
                id,
                from,
                to,
                kind,
                confidence,
                evidence,
            });
        }
    }

    fn finish(mut self) -> TopologyGraph {
        self.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        self.edges.sort_by(|left, right| left.id.cmp(&right.id));
        TopologyGraph {
            generated_at_ms: now_ms(),
            nodes: self.nodes,
            edges: self.edges,
            issues: self.issues,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        BatterySnapshot, BindingScope, FeatureAvailability, ManagedCommandState, PortProtocol,
        ProcessKey, ProjectScanHealth, ProjectScanState, ProtectedState, ReachabilityState,
        SystemSnapshot,
    };

    use super::*;

    #[test]
    fn managed_project_process_port_url_chain_has_evidence() {
        let project = Project {
            id: "project-1".to_owned(),
            name: "Fixture".to_owned(),
            root_path: "C:\\fixtures\\app".to_owned(),
            canonical_root_path: "C:\\fixtures\\app".to_owned(),
            tags: Vec::new(),
            notes: String::new(),
            checklist: Vec::new(),
            pinned: false,
            archived: false,
            detected_stacks: Vec::new(),
            manifests: Vec::new(),
            package_manager: None,
            scripts: Vec::new(),
            git_summary: None,
            compose_files: Vec::new(),
            environment_files: Vec::new(),
            local_database_hints: Vec::new(),
            last_scanned_at: Some(1),
            scan_health: ProjectScanHealth {
                state: ProjectScanState::Healthy,
                issues: Vec::new(),
            },
        };
        let process = ProcessSnapshot {
            key: ProcessKey {
                pid: 42,
                start_time: 10,
            },
            parent_pid: None,
            name: "node.exe".to_owned(),
            executable_path: None,
            cwd: Some("C:\\fixtures\\app".to_owned()),
            command_line_redacted: None,
            status: "run".to_owned(),
            cpu_percent: 0.0,
            memory_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            protected_state: ProtectedState::Accessible,
            managed_by_mr_manager: true,
            listening_port_count: 1,
        };
        let port = PortEndpoint {
            protocol: PortProtocol::Tcp,
            local_address: "127.0.0.1".to_owned(),
            local_port: 4173,
            state: "listen".to_owned(),
            owning_process_key: Some(process.key),
            owning_process_name: Some(process.name.clone()),
            binding_scope: BindingScope::Loopback,
            inferred_scheme: Some("http".to_owned()),
            local_url: Some("http://127.0.0.1:4173".to_owned()),
            lan_urls: Vec::new(),
            reachability_state: ReachabilityState::NotTested,
            evidence: vec!["Windows ownership table maps this endpoint to PID 42.".to_owned()],
        };
        let run = ManagedCommand {
            run_id: "run-1".to_owned(),
            project_id: "project-1".to_owned(),
            script_id: "node:dev".to_owned(),
            label: "dev".to_owned(),
            executable: "npm.cmd".to_owned(),
            arguments: vec!["run".to_owned(), "dev".to_owned()],
            working_directory: "C:\\fixtures\\app".to_owned(),
            pid: Some(42),
            started_at_ms: 1,
            ended_at_ms: None,
            state: ManagedCommandState::Running,
            exit_code: None,
            stop_requested: false,
            log_count: 1,
        };

        let graph = build_topology(
            CollectedSnapshot {
                overview: empty_overview(),
                processes: vec![process],
                ports: vec![port],
            },
            vec![project],
            vec![run],
        );

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == TopologyNodeKind::Url)
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == TopologyEdgeKind::PortExposesUrl
                    && edge.confidence == TopologyConfidence::Inferred
                    && !edge.evidence.is_empty())
        );
    }

    #[test]
    fn unregistered_live_service_is_visible_without_fabricated_project_edge() {
        let process = ProcessSnapshot {
            key: ProcessKey {
                pid: 77,
                start_time: 20,
            },
            parent_pid: None,
            name: "python.exe".to_owned(),
            executable_path: None,
            cwd: None,
            command_line_redacted: Some("python -m http.server 8000".to_owned()),
            status: "run".to_owned(),
            cpu_percent: 0.0,
            memory_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            protected_state: ProtectedState::Accessible,
            managed_by_mr_manager: false,
            listening_port_count: 1,
        };
        let port = PortEndpoint {
            protocol: PortProtocol::Tcp,
            local_address: "127.0.0.1".to_owned(),
            local_port: 8000,
            state: "listen".to_owned(),
            owning_process_key: Some(process.key),
            owning_process_name: Some(process.name.clone()),
            binding_scope: BindingScope::Loopback,
            inferred_scheme: Some("http".to_owned()),
            local_url: Some("http://127.0.0.1:8000".to_owned()),
            lan_urls: Vec::new(),
            reachability_state: ReachabilityState::NotTested,
            evidence: vec!["Windows ownership table maps this endpoint to PID 77.".to_owned()],
        };
        let graph = build_topology(
            CollectedSnapshot {
                overview: empty_overview(),
                processes: vec![process],
                ports: vec![port],
            },
            Vec::new(),
            Vec::new(),
        );

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == TopologyNodeKind::Process)
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == TopologyNodeKind::Port)
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == TopologyNodeKind::Url)
        );
        assert!(
            !graph
                .edges
                .iter()
                .any(|edge| { edge.kind == TopologyEdgeKind::ProjectContainsProcess })
        );
        assert!(
            graph
                .issues
                .iter()
                .any(|issue| issue.code == "UNASSOCIATED_LIVE_SERVICES")
        );
    }

    fn empty_overview() -> crate::domain::OverviewSnapshot {
        crate::domain::OverviewSnapshot {
            system: SystemSnapshot {
                collected_at_ms: 0,
                sequence: 0,
                operating_system: "windows".to_owned(),
                operating_system_version: None,
                kernel_version: None,
                host_name: None,
                uptime_seconds: 0,
                cpu: crate::domain::CpuSnapshot {
                    total_usage_percent: 0.0,
                    logical_core_count: 0,
                    physical_core_count: None,
                    per_core_usage_percent: Vec::new(),
                },
                memory: crate::domain::MemorySnapshot {
                    total_bytes: 0,
                    used_bytes: 0,
                    available_bytes: 0,
                    swap_total_bytes: 0,
                    swap_used_bytes: 0,
                },
                disks: Vec::new(),
                network: crate::domain::NetworkThroughputSnapshot {
                    received_bytes_per_second: 0,
                    transmitted_bytes_per_second: 0,
                    total_received_bytes: 0,
                    total_transmitted_bytes: 0,
                },
                battery: BatterySnapshot {
                    availability: FeatureAvailability::unsupported("fixture"),
                    percentage: None,
                    ac_online: None,
                    remaining_seconds: None,
                },
                gpu: FeatureAvailability::unsupported("fixture"),
                issues: Vec::new(),
            },
            processes: crate::domain::ProcessSummary {
                total: 0,
                accessible: 0,
                top_cpu: Vec::new(),
                top_memory: Vec::new(),
            },
            ports: crate::domain::PortSummary {
                total_listening: 0,
                development_listeners: 0,
                endpoints: Vec::new(),
            },
            collector_issues: Vec::new(),
        }
    }
}
