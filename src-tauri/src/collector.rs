use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::LazyLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use sysinfo::{Disks, Networks, System};

use crate::domain::{
    BatterySnapshot, BindingScope, CollectorIssue, CpuSnapshot, DiskSnapshot, FeatureAvailability,
    MemorySnapshot, NetworkThroughputSnapshot, OverviewSnapshot, PortEndpoint, PortProtocol,
    PortSummary, ProcessKey, ProcessSnapshot, ProcessSummary, ProtectedState, ReachabilityState,
    SystemSnapshot,
};
use crate::platform::{self, OwnedPort, TransportProtocol};
use crate::security::redaction;

static DEFAULT_LAN_IPV4: LazyLock<Option<Ipv4Addr>> = LazyLock::new(default_lan_ipv4);

pub struct CollectedSnapshot {
    pub overview: OverviewSnapshot,
    pub processes: Vec<ProcessSnapshot>,
    pub ports: Vec<PortEndpoint>,
}

pub struct SystemCollector {
    system: System,
    disks: Disks,
    networks: Networks,
    sequence: u64,
    last_network_refresh: Instant,
}

impl SystemCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            sequence: 0,
            last_network_refresh: Instant::now(),
        }
    }

    pub fn collect(&mut self) -> CollectedSnapshot {
        self.system.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.sequence = self.sequence.wrapping_add(1);

        let mut issues = Vec::new();
        let owned_ports = match platform::query_owned_ports() {
            Ok(ports) => ports,
            Err(error) => {
                issues.push(
                    CollectorIssue::new("PORT_ENUMERATION_FAILED", error.to_string())
                        .with_remediation(
                            "Some protected endpoints may require additional Windows access.",
                        )
                        .permission_relevant(),
                );
                Vec::new()
            }
        };

        let mut processes = self.process_snapshots();
        let process_index: HashMap<u32, (ProcessKey, String)> = processes
            .iter()
            .map(|process| (process.key.pid, (process.key, process.name.clone())))
            .collect();
        let ports = owned_ports
            .into_iter()
            .map(|port| port_endpoint(port, &process_index))
            .collect::<Vec<_>>();

        let mut port_counts = HashMap::<u32, u32>::new();
        for endpoint in &ports {
            if let Some(key) = endpoint.owning_process_key {
                *port_counts.entry(key.pid).or_default() += 1;
            }
        }
        for process in &mut processes {
            process.listening_port_count = port_counts.get(&process.key.pid).copied().unwrap_or(0);
        }

        let system = self.system_snapshot(issues.clone());
        let mut top_cpu = processes.clone();
        top_cpu.sort_by(|left, right| right.cpu_percent.total_cmp(&left.cpu_percent));
        top_cpu.truncate(12);
        let mut top_memory = processes.clone();
        top_memory.sort_by(|left, right| right.memory_bytes.cmp(&left.memory_bytes));
        top_memory.truncate(12);

        let overview = OverviewSnapshot {
            system,
            processes: ProcessSummary {
                total: u32::try_from(processes.len()).unwrap_or(u32::MAX),
                accessible: u32::try_from(
                    processes
                        .iter()
                        .filter(|process| process.protected_state == ProtectedState::Accessible)
                        .count(),
                )
                .unwrap_or(u32::MAX),
                top_cpu,
                top_memory,
            },
            ports: PortSummary {
                total_listening: u32::try_from(ports.len()).unwrap_or(u32::MAX),
                development_listeners: u32::try_from(
                    ports
                        .iter()
                        .filter(|endpoint| endpoint.local_url.is_some())
                        .count(),
                )
                .unwrap_or(u32::MAX),
                endpoints: ports.iter().take(40).cloned().collect(),
            },
            collector_issues: issues,
        };

        CollectedSnapshot {
            overview,
            processes,
            ports,
        }
    }

    fn process_snapshots(&self) -> Vec<ProcessSnapshot> {
        let mut processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let executable_path = process
                    .exe()
                    .map(display_path)
                    .filter(|path| !path.is_empty());
                let cwd = process
                    .cwd()
                    .map(display_path)
                    .filter(|path| !path.is_empty());
                let command_line = if process.cmd().is_empty() {
                    None
                } else {
                    Some(redaction::redact(
                        &process
                            .cmd()
                            .iter()
                            .map(|value| value.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ))
                };
                let protected_state =
                    if executable_path.is_some() || cwd.is_some() || command_line.is_some() {
                        ProtectedState::Accessible
                    } else {
                        ProtectedState::Unknown
                    };
                let disk = process.disk_usage();

                ProcessSnapshot {
                    key: ProcessKey {
                        pid: pid.as_u32(),
                        start_time: process.start_time(),
                    },
                    parent_pid: process.parent().map(|parent| parent.as_u32()),
                    name: process.name().to_string_lossy().into_owned(),
                    executable_path,
                    cwd,
                    command_line_redacted: command_line,
                    status: format!("{:?}", process.status()).to_lowercase(),
                    cpu_percent: f64::from(process.cpu_usage()),
                    memory_bytes: process.memory(),
                    disk_read_bytes: disk.total_read_bytes,
                    disk_write_bytes: disk.total_written_bytes,
                    protected_state,
                    managed_by_mr_manager: false,
                    listening_port_count: 0,
                }
            })
            .collect::<Vec<_>>();
        processes.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.key.pid.cmp(&right.key.pid))
        });
        processes
    }

    fn system_snapshot(&mut self, mut issues: Vec<CollectorIssue>) -> SystemSnapshot {
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_network_refresh)
            .as_secs_f64()
            .max(0.001);
        self.last_network_refresh = now;
        let received_delta = self
            .networks
            .values()
            .map(|data| data.received())
            .sum::<u64>();
        let transmitted_delta = self
            .networks
            .values()
            .map(|data| data.transmitted())
            .sum::<u64>();

        let battery = match platform::query_power_status() {
            Ok(status) if status.battery_present => BatterySnapshot {
                availability: FeatureAvailability::available(match status.ac_online {
                    Some(true) => "Battery detected; AC power is connected.",
                    Some(false) => "Battery detected; the system is running on battery.",
                    None => "Battery detected; AC power state is unknown.",
                }),
                percentage: status.battery_percent,
                ac_online: status.ac_online,
                remaining_seconds: status.remaining_seconds,
            },
            Ok(_) => BatterySnapshot {
                availability: FeatureAvailability::unsupported(
                    "Windows reports no system battery.",
                ),
                percentage: None,
                ac_online: None,
                remaining_seconds: None,
            },
            Err(error) => {
                issues.push(CollectorIssue::new(
                    "POWER_STATUS_UNAVAILABLE",
                    error.to_string(),
                ));
                BatterySnapshot {
                    availability: FeatureAvailability::error(
                        "Windows power status could not be read.",
                        "Retry the collector without elevating the whole application.",
                    ),
                    percentage: None,
                    ac_online: None,
                    remaining_seconds: None,
                }
            }
        };

        SystemSnapshot {
            collected_at_ms: unix_time_ms(),
            sequence: self.sequence,
            operating_system: System::name().unwrap_or_else(|| std::env::consts::OS.to_owned()),
            operating_system_version: System::long_os_version(),
            kernel_version: System::kernel_version(),
            host_name: System::host_name(),
            uptime_seconds: System::uptime(),
            cpu: CpuSnapshot {
                total_usage_percent: f64::from(self.system.global_cpu_usage()),
                logical_core_count: u32::try_from(self.system.cpus().len()).unwrap_or(u32::MAX),
                physical_core_count: System::physical_core_count()
                    .and_then(|count| u32::try_from(count).ok()),
                per_core_usage_percent: self
                    .system
                    .cpus()
                    .iter()
                    .map(|cpu| f64::from(cpu.cpu_usage()))
                    .collect(),
            },
            memory: MemorySnapshot {
                total_bytes: self.system.total_memory(),
                used_bytes: self.system.used_memory(),
                available_bytes: self.system.available_memory(),
                swap_total_bytes: self.system.total_swap(),
                swap_used_bytes: self.system.used_swap(),
            },
            disks: self
                .disks
                .list()
                .iter()
                .map(|disk| DiskSnapshot {
                    name: disk.name().to_string_lossy().into_owned(),
                    mount_point: display_path(disk.mount_point()),
                    kind: format!("{:?}", disk.kind()).to_lowercase(),
                    total_bytes: disk.total_space(),
                    available_bytes: disk.available_space(),
                    removable: disk.is_removable(),
                })
                .collect(),
            network: NetworkThroughputSnapshot {
                received_bytes_per_second: (received_delta as f64 / elapsed) as u64,
                transmitted_bytes_per_second: (transmitted_delta as f64 / elapsed) as u64,
                total_received_bytes: self
                    .networks
                    .values()
                    .map(|data| data.total_received())
                    .sum(),
                total_transmitted_bytes: self
                    .networks
                    .values()
                    .map(|data| data.total_transmitted())
                    .sum(),
            },
            battery,
            gpu: FeatureAvailability::unsupported(
                "GPU telemetry is shown through the System Diagnostics provider when supported.",
            ),
            issues,
        }
    }
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn port_endpoint(
    port: OwnedPort,
    process_index: &HashMap<u32, (ProcessKey, String)>,
) -> PortEndpoint {
    let binding_scope = binding_scope(port.local_address);
    let inferred_scheme = infer_scheme(port.local_port).map(str::to_owned);
    let local_url = inferred_scheme.as_ref().map(|scheme| {
        let host = if port.local_address.is_unspecified() {
            "localhost".to_owned()
        } else if port.local_address.is_ipv6() {
            format!("[{}]", port.local_address)
        } else {
            port.local_address.to_string()
        };
        format!("{scheme}://{host}:{}", port.local_port)
    });
    let lan_urls = inferred_scheme
        .as_deref()
        .map(|scheme| lan_url_candidates(scheme, port.local_address, port.local_port))
        .unwrap_or_default();
    let owner = process_index.get(&port.owning_pid);
    let mut evidence = vec![format!(
        "Windows ownership table maps this endpoint to PID {}.",
        port.owning_pid
    )];
    if inferred_scheme.is_some() {
        evidence.push(
            "The URL scheme is inferred from a common development port and is not verified."
                .to_owned(),
        );
    }

    PortEndpoint {
        protocol: match port.protocol {
            TransportProtocol::Tcp => PortProtocol::Tcp,
            TransportProtocol::Udp => PortProtocol::Udp,
        },
        local_address: port.local_address.to_string(),
        local_port: port.local_port,
        state: port.state.to_owned(),
        owning_process_key: owner.map(|(key, _)| *key),
        owning_process_name: owner.map(|(_, name)| name.clone()),
        binding_scope,
        inferred_scheme,
        local_url,
        lan_urls,
        reachability_state: ReachabilityState::NotTested,
        evidence,
    }
}

fn lan_url_candidates(scheme: &str, address: IpAddr, port: u16) -> Vec<String> {
    match address {
        IpAddr::V4(ip) if is_private_lan_ipv4(ip) => vec![format!("{scheme}://{ip}:{port}")],
        IpAddr::V4(ip) if ip.is_unspecified() => DEFAULT_LAN_IPV4
            .as_ref()
            .map(|ip| vec![format!("{scheme}://{ip}:{port}")])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn default_lan_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 80)).ok()?;
    let local = socket.local_addr().ok()?.ip();
    match local {
        IpAddr::V4(ip) if is_private_lan_ipv4(ip) => Some(ip),
        _ => None,
    }
}

fn is_private_lan_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private() || ip.octets()[0] == 169 && ip.octets()[1] == 254
}

fn binding_scope(address: IpAddr) -> BindingScope {
    if address.is_loopback() {
        BindingScope::Loopback
    } else if address.is_unspecified() {
        BindingScope::AllInterfaces
    } else {
        BindingScope::SpecificInterface
    }
}

fn infer_scheme(port: u16) -> Option<&'static str> {
    match port {
        80 | 3000 | 3001 | 4173 | 4200 | 5000 | 5173 | 5174 | 8000 | 8080 | 8787 => Some("http"),
        443 | 8443 => Some("https"),
        _ => None,
    }
}

fn display_path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_scope_is_evidence_based() {
        assert_eq!(
            binding_scope("127.0.0.1".parse().unwrap()),
            BindingScope::Loopback
        );
        assert_eq!(
            binding_scope("0.0.0.0".parse().unwrap()),
            BindingScope::AllInterfaces
        );
        assert_eq!(
            binding_scope("192.168.1.20".parse().unwrap()),
            BindingScope::SpecificInterface
        );
    }

    #[test]
    fn url_inference_is_deliberately_narrow() {
        assert_eq!(infer_scheme(5173), Some("http"));
        assert_eq!(infer_scheme(443), Some("https"));
        assert_eq!(infer_scheme(5432), None);
    }

    #[test]
    fn lan_candidates_are_private_or_specific_only() {
        assert_eq!(
            lan_url_candidates("http", IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)), 5173),
            vec!["http://192.168.1.20:5173"]
        );
        assert!(lan_url_candidates("http", IpAddr::V4(Ipv4Addr::LOCALHOST), 5173).is_empty());
        assert!(lan_url_candidates("http", "8.8.8.8".parse().unwrap(), 5173).is_empty());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn live_standard_user_collection_returns_real_process_and_memory_data() {
        let mut collector = SystemCollector::new();
        let snapshot = collector.collect();

        assert!(snapshot.overview.system.memory.total_bytes > 0);
        assert!(snapshot.overview.system.cpu.logical_core_count > 0);
        assert!(!snapshot.processes.is_empty());
        assert!(
            snapshot
                .processes
                .iter()
                .any(|process| process.key.pid == std::process::id())
        );
    }
}
