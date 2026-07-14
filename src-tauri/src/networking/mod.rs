use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpStream, ToSocketAddrs};
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sysinfo::Networks;
use thiserror::Error;

use crate::domain::{
    AdapterThroughput, BindingScope, DnsStatus, FeatureAvailability, GatewayStatus,
    LocalDevServerWarning, NetworkAdapterSnapshot, NetworkDashboardSnapshot, NetworkDiagnosticKind,
    NetworkDiagnosticReport, NetworkDiagnosticRequest, NetworkDiagnosticResult,
    NetworkDiagnosticState, NetworkEvidence, NetworkTimelinePoint, PerProcessNetworkUsage,
    PortEndpoint, ProcessSnapshot, VpnState,
};
use crate::platform::{self, NetworkAdapterInfo};
use crate::security::redaction;

const TIMELINE_LIMIT: usize = 180;
const GATEWAY_PING_TIMEOUT: Duration = Duration::from_millis(1_500);
const NETSH_TIMEOUT: Duration = Duration::from_millis(1_500);
const INTERNET_TIMEOUT: Duration = Duration::from_millis(3_000);
const SMALL_DOWNLOAD_LIMIT: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("external internet diagnostics are disabled")]
    ExternalDiagnosticsDisabled,
    #[error("external internet diagnostics require explicit consent")]
    ExternalConsentMissing,
    #[error("network command timed out")]
    Timeout,
    #[error("network command could not run")]
    Io(#[from] io::Error),
}

#[derive(Debug)]
struct AdapterHistory {
    baseline_received: u64,
    baseline_transmitted: u64,
    peak_received_bps: u64,
    peak_transmitted_bps: u64,
    timeline: VecDeque<NetworkTimelinePoint>,
}

impl AdapterHistory {
    fn new(total_received: u64, total_transmitted: u64) -> Self {
        Self {
            baseline_received: total_received,
            baseline_transmitted: total_transmitted,
            peak_received_bps: 0,
            peak_transmitted_bps: 0,
            timeline: VecDeque::new(),
        }
    }

    fn throughput(
        &mut self,
        collected_at_ms: u64,
        received_bps: u64,
        transmitted_bps: u64,
        total_received: u64,
        total_transmitted: u64,
    ) -> AdapterThroughput {
        self.peak_received_bps = self.peak_received_bps.max(received_bps);
        self.peak_transmitted_bps = self.peak_transmitted_bps.max(transmitted_bps);
        self.timeline.push_back(NetworkTimelinePoint {
            collected_at_ms,
            received_bytes_per_second: received_bps,
            transmitted_bytes_per_second: transmitted_bps,
        });
        while self.timeline.len() > TIMELINE_LIMIT {
            self.timeline.pop_front();
        }

        AdapterThroughput {
            received_bytes_per_second: received_bps,
            transmitted_bytes_per_second: transmitted_bps,
            session_received_bytes: total_received.saturating_sub(self.baseline_received),
            session_transmitted_bytes: total_transmitted.saturating_sub(self.baseline_transmitted),
            total_received_bytes: total_received,
            total_transmitted_bytes: total_transmitted,
            peak_received_bytes_per_second: self.peak_received_bps,
            peak_transmitted_bytes_per_second: self.peak_transmitted_bps,
            timeline: self.timeline.iter().cloned().collect(),
        }
    }
}

#[derive(Debug)]
pub struct NetworkMonitor {
    networks: Networks,
    last_refresh: Instant,
    histories: HashMap<String, AdapterHistory>,
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            last_refresh: Instant::now(),
            histories: HashMap::new(),
        }
    }

    pub fn snapshot(
        &mut self,
        processes: &[ProcessSnapshot],
        ports: &[PortEndpoint],
        external_diagnostics_enabled: bool,
    ) -> NetworkDashboardSnapshot {
        let collected_at_ms = unix_time_ms();
        let elapsed = self.refresh_elapsed();
        let throughput = self.collect_throughput(collected_at_ms, elapsed);
        let adapters = platform::query_network_adapters().unwrap_or_default();
        let dns_server_count = adapters
            .iter()
            .map(|adapter| adapter.dns_server_count)
            .sum::<u32>();
        let wifi_signals = wifi_signal_quality();
        let lan_ip_candidates = lan_candidates(&adapters);
        let adapter_snapshots = self.adapter_snapshots(
            &adapters,
            &throughput,
            collected_at_ms,
            &lan_ip_candidates,
            &wifi_signals,
        );
        let gateway_reachability = gateway_status(&adapters);
        let dns_status = dns_status(dns_server_count);
        let vpn_state = vpn_state(&adapters, processes);
        let local_dev_server_warnings = local_dev_warnings(ports);
        let combined = throughput
            .get(COMBINED_KEY)
            .cloned()
            .unwrap_or_else(empty_throughput);

        NetworkDashboardSnapshot {
            collected_at_ms,
            external_diagnostics_enabled,
            combined,
            adapters: adapter_snapshots,
            gateway_reachability,
            dns_status,
            vpn_state,
            lan_ip_candidates,
            local_dev_server_warnings,
            per_process_usage: PerProcessNetworkUsage {
                availability: FeatureAvailability::unsupported(
                    "Reliable standard-user per-process network throughput is not available through this Phase 5 Windows adapter. Mr Manager will not guess from socket ownership.",
                ),
                entries: Vec::new(),
            },
            privacy_note: "Default network data redacts DNS server addresses, MAC addresses, SSIDs, public IP addresses, remote endpoints, and history exports.".to_owned(),
        }
    }

    pub fn run_diagnostic(
        &mut self,
        request: NetworkDiagnosticRequest,
        external_diagnostics_enabled: bool,
        processes: &[ProcessSnapshot],
        ports: &[PortEndpoint],
    ) -> Result<NetworkDiagnosticReport, NetworkError> {
        let started_at_ms = unix_time_ms();
        let is_external = matches!(
            request.kind,
            NetworkDiagnosticKind::InternetLatency
                | NetworkDiagnosticKind::InternetDnsResolution
                | NetworkDiagnosticKind::PacketLoss
                | NetworkDiagnosticKind::DownloadSpeed
                | NetworkDiagnosticKind::UploadSpeed
        );
        if is_external && !external_diagnostics_enabled {
            return Err(NetworkError::ExternalDiagnosticsDisabled);
        }
        if is_external && !request.consent_to_external {
            return Err(NetworkError::ExternalConsentMissing);
        }

        let snapshot = self.snapshot(processes, ports, external_diagnostics_enabled);
        let mut endpoints_contacted = Vec::new();
        let mut warnings = Vec::new();
        let mut results = Vec::new();

        match request.kind {
            NetworkDiagnosticKind::GatewayReachability => {
                results.push(NetworkDiagnosticResult {
                    label: "Gateway reachability".to_owned(),
                    state: snapshot.gateway_reachability.state.clone(),
                    value: snapshot.gateway_reachability.latency_ms.map(|latency| {
                        format!(
                            "{} ms to {}",
                            latency,
                            snapshot
                                .gateway_reachability
                                .gateway
                                .as_deref()
                                .unwrap_or("configured gateway")
                        )
                    }),
                    local_only: true,
                    contacted_internet: false,
                    evidence: snapshot.gateway_reachability.evidence,
                });
            }
            NetworkDiagnosticKind::DnsStatus => {
                results.push(NetworkDiagnosticResult {
                    label: "DNS configuration".to_owned(),
                    state: snapshot.dns_status.state.clone(),
                    value: Some(format!(
                        "{} configured DNS server(s); addresses redacted",
                        snapshot.dns_status.configured_server_count
                    )),
                    local_only: true,
                    contacted_internet: false,
                    evidence: snapshot.dns_status.evidence,
                });
            }
            NetworkDiagnosticKind::InternetLatency => {
                endpoints_contacted.push("example.com:80".to_owned());
                results.push(internet_latency_result());
            }
            NetworkDiagnosticKind::InternetDnsResolution => {
                endpoints_contacted.push("DNS resolution for example.com".to_owned());
                results.push(internet_dns_result());
            }
            NetworkDiagnosticKind::PacketLoss => {
                endpoints_contacted.push("example.com via ping.exe".to_owned());
                results.push(packet_loss_result());
            }
            NetworkDiagnosticKind::DownloadSpeed => {
                endpoints_contacted.push("example.com:80".to_owned());
                warnings.push(
                    "This is a small explicit download probe, not a full multi-server speed test."
                        .to_owned(),
                );
                results.push(download_probe_result());
            }
            NetworkDiagnosticKind::UploadSpeed => {
                warnings.push(
                    "Upload speed is marked unavailable because no trusted upload endpoint is configured in Phase 5."
                        .to_owned(),
                );
                results.push(NetworkDiagnosticResult {
                    label: "Upload speed".to_owned(),
                    state: NetworkDiagnosticState::Unavailable,
                    value: None,
                    local_only: false,
                    contacted_internet: false,
                    evidence: vec![NetworkEvidence {
                        source: "safety-boundary".to_owned(),
                        detail:
                            "Mr Manager will not upload arbitrary data to an unspecified server."
                                .to_owned(),
                    }],
                });
            }
            NetworkDiagnosticKind::RouteVpnBehavior => {
                results.push(NetworkDiagnosticResult {
                    label: "Route / VPN behaviour".to_owned(),
                    state: if snapshot.vpn_state.likely_active {
                        NetworkDiagnosticState::Warn
                    } else {
                        NetworkDiagnosticState::Pass
                    },
                    value: Some(snapshot.vpn_state.label),
                    local_only: true,
                    contacted_internet: false,
                    evidence: snapshot.vpn_state.evidence,
                });
            }
        }

        Ok(NetworkDiagnosticReport {
            kind: request.kind,
            started_at_ms,
            completed_at_ms: unix_time_ms(),
            local_only: results.iter().all(|result| result.local_only),
            contacted_internet: results.iter().any(|result| result.contacted_internet),
            endpoints_contacted,
            results,
            warnings,
        })
    }

    fn refresh_elapsed(&mut self) -> f64 {
        self.networks.refresh(true);
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.last_refresh)
            .as_secs_f64()
            .max(0.001);
        self.last_refresh = now;
        elapsed
    }

    fn collect_throughput(
        &mut self,
        collected_at_ms: u64,
        elapsed_seconds: f64,
    ) -> HashMap<String, AdapterThroughput> {
        let mut throughput = HashMap::new();
        let mut combined_received_bps = 0u64;
        let mut combined_transmitted_bps = 0u64;
        let mut combined_total_received = 0u64;
        let mut combined_total_transmitted = 0u64;

        for (name, data) in &self.networks {
            let received_bps = (data.received() as f64 / elapsed_seconds) as u64;
            let transmitted_bps = (data.transmitted() as f64 / elapsed_seconds) as u64;
            let total_received = data.total_received();
            let total_transmitted = data.total_transmitted();
            combined_received_bps = combined_received_bps.saturating_add(received_bps);
            combined_transmitted_bps = combined_transmitted_bps.saturating_add(transmitted_bps);
            combined_total_received = combined_total_received.saturating_add(total_received);
            combined_total_transmitted =
                combined_total_transmitted.saturating_add(total_transmitted);

            let key = normalize_adapter_key(name);
            let history = self
                .histories
                .entry(key.clone())
                .or_insert_with(|| AdapterHistory::new(total_received, total_transmitted));
            throughput.insert(
                key,
                history.throughput(
                    collected_at_ms,
                    received_bps,
                    transmitted_bps,
                    total_received,
                    total_transmitted,
                ),
            );
        }

        let combined_history = self
            .histories
            .entry(COMBINED_KEY.to_owned())
            .or_insert_with(|| {
                AdapterHistory::new(combined_total_received, combined_total_transmitted)
            });
        throughput.insert(
            COMBINED_KEY.to_owned(),
            combined_history.throughput(
                collected_at_ms,
                combined_received_bps,
                combined_transmitted_bps,
                combined_total_received,
                combined_total_transmitted,
            ),
        );
        throughput
    }

    fn adapter_snapshots(
        &mut self,
        adapters: &[NetworkAdapterInfo],
        throughput: &HashMap<String, AdapterThroughput>,
        collected_at_ms: u64,
        lan_ip_candidates: &[String],
        wifi_signals: &HashMap<String, u8>,
    ) -> Vec<NetworkAdapterSnapshot> {
        let mut snapshots = adapters
            .iter()
            .map(|adapter| {
                let key = normalize_adapter_key(&adapter.name);
                let throughput = throughput
                    .get(&key)
                    .cloned()
                    .or_else(|| {
                        adapter
                            .description
                            .as_ref()
                            .and_then(|description| throughput.get(&normalize_adapter_key(description)).cloned())
                    })
                    .unwrap_or_else(|| {
                        self.histories
                            .entry(key.clone())
                            .or_insert_with(|| AdapterHistory::new(0, 0))
                            .throughput(collected_at_ms, 0, 0, 0, 0)
                    });
                NetworkAdapterSnapshot {
                    id: adapter.id.clone(),
                    name: adapter.name.clone(),
                    display_name: adapter.name.clone(),
                    description: adapter.description.clone(),
                    adapter_type: adapter.adapter_type.clone(),
                    operational_status: adapter.operational_status.clone(),
                    ipv4_addresses: adapter
                        .ipv4_addresses
                        .iter()
                        .map(safe_ip_display)
                        .collect(),
                    ipv6_addresses: adapter
                        .ipv6_addresses
                        .iter()
                        .map(safe_ip_display)
                        .collect(),
                    gateway_addresses: adapter
                        .gateway_addresses
                        .iter()
                        .map(safe_ip_display)
                        .collect(),
                    dns_server_count: adapter.dns_server_count,
                    link_speed_bits_per_second: adapter
                        .receive_link_speed_bits_per_second
                        .or(adapter.transmit_link_speed_bits_per_second),
                    wifi_signal_quality_percent: wifi_signals
                        .get(&normalize_adapter_key(&adapter.name))
                        .copied(),
                    interface_metric: adapter.ipv4_metric.or(adapter.ipv6_metric),
                    lan_ip_candidates: adapter
                        .ipv4_addresses
                        .iter()
                        .filter_map(lan_candidate)
                        .collect(),
                    throughput,
                    evidence: vec![NetworkEvidence {
                        source: "windows-ip-helper".to_owned(),
                        detail: "Adapter metadata came from GetAdaptersAddresses; MAC, SSID, DNS addresses, and public IPs are not exposed.".to_owned(),
                    }],
                }
            })
            .collect::<Vec<_>>();

        if snapshots.is_empty() && !lan_ip_candidates.is_empty() {
            snapshots.push(NetworkAdapterSnapshot {
                id: "local-addresses".to_owned(),
                name: "Local addresses".to_owned(),
                display_name: "Local addresses".to_owned(),
                description: Some(
                    "Fallback adapter record from local network counters.".to_owned(),
                ),
                adapter_type: "unknown".to_owned(),
                operational_status: "unknown".to_owned(),
                ipv4_addresses: lan_ip_candidates.to_vec(),
                ipv6_addresses: Vec::new(),
                gateway_addresses: Vec::new(),
                dns_server_count: 0,
                link_speed_bits_per_second: None,
                wifi_signal_quality_percent: None,
                interface_metric: None,
                lan_ip_candidates: lan_ip_candidates.to_vec(),
                throughput: throughput
                    .get(COMBINED_KEY)
                    .cloned()
                    .unwrap_or_else(empty_throughput),
                evidence: vec![NetworkEvidence {
                    source: "sysinfo-network-counters".to_owned(),
                    detail: "Only throughput counters were available for this fallback record."
                        .to_owned(),
                }],
            });
        }

        snapshots.sort_by(|left, right| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
        });
        snapshots
    }
}

const COMBINED_KEY: &str = "__combined__";

fn empty_throughput() -> AdapterThroughput {
    AdapterThroughput {
        received_bytes_per_second: 0,
        transmitted_bytes_per_second: 0,
        session_received_bytes: 0,
        session_transmitted_bytes: 0,
        total_received_bytes: 0,
        total_transmitted_bytes: 0,
        peak_received_bytes_per_second: 0,
        peak_transmitted_bytes_per_second: 0,
        timeline: Vec::new(),
    }
}

fn lan_candidates(adapters: &[NetworkAdapterInfo]) -> Vec<String> {
    let mut candidates = adapters
        .iter()
        .flat_map(|adapter| adapter.ipv4_addresses.iter())
        .filter_map(lan_candidate)
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    candidates
}

fn lan_candidate(ip: &IpAddr) -> Option<String> {
    match ip {
        IpAddr::V4(ip) if is_lan_ipv4(*ip) => Some(ip.to_string()),
        _ => None,
    }
}

fn is_lan_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private() || (ip.octets()[0] == 169 && ip.octets()[1] == 254)
}

fn safe_ip_display(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) if ip.is_loopback() || is_lan_ipv4(*ip) => ip.to_string(),
        IpAddr::V6(ip) if ip.is_loopback() || ip.is_unique_local() => ip.to_string(),
        IpAddr::V4(_) => "redacted-public-ipv4".to_owned(),
        IpAddr::V6(_) => "redacted-public-ipv6".to_owned(),
    }
}

fn gateway_status(adapters: &[NetworkAdapterInfo]) -> GatewayStatus {
    let gateway = adapters
        .iter()
        .flat_map(|adapter| adapter.gateway_addresses.iter())
        .find(|ip| !ip.is_loopback())
        .cloned();
    let Some(gateway) = gateway else {
        return GatewayStatus {
            state: NetworkDiagnosticState::Unavailable,
            gateway: None,
            latency_ms: None,
            local_only: true,
            evidence: vec![NetworkEvidence {
                source: "windows-ip-helper".to_owned(),
                detail: "No gateway address was returned by the adapter inventory.".to_owned(),
            }],
        };
    };

    let started = Instant::now();
    match ping_host(&gateway.to_string()) {
        Ok(true) => GatewayStatus {
            state: NetworkDiagnosticState::Pass,
            gateway: Some(safe_ip_display(&gateway)),
            latency_ms: Some(started.elapsed().as_millis() as u64),
            local_only: true,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: "Ran exact local gateway reachability check with one packet.".to_owned(),
            }],
        },
        Ok(false) => GatewayStatus {
            state: NetworkDiagnosticState::Warn,
            gateway: Some(safe_ip_display(&gateway)),
            latency_ms: None,
            local_only: true,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: "Gateway is configured, but ping.exe did not confirm reachability."
                    .to_owned(),
            }],
        },
        Err(error) => GatewayStatus {
            state: NetworkDiagnosticState::Unavailable,
            gateway: Some(safe_ip_display(&gateway)),
            latency_ms: None,
            local_only: true,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: format!("Gateway check could not run: {error}"),
            }],
        },
    }
}

fn dns_status(configured_server_count: u32) -> DnsStatus {
    let state = if configured_server_count > 0 {
        NetworkDiagnosticState::Pass
    } else {
        NetworkDiagnosticState::Warn
    };
    DnsStatus {
        state,
        local_only: true,
        configured_server_count,
        evidence: vec![NetworkEvidence {
            source: "windows-ip-helper".to_owned(),
            detail: "DNS server addresses are redacted; only the configured count is exposed."
                .to_owned(),
        }],
    }
}

fn wifi_signal_quality() -> HashMap<String, u8> {
    match run_netsh_wlan_interfaces() {
        Ok(text) => parse_netsh_wlan_signal(&text),
        Err(_) => HashMap::new(),
    }
}

fn run_netsh_wlan_interfaces() -> Result<String, NetworkError> {
    let mut child = crate::platform::process::hidden_command("netsh.exe")
        .args(["wlan", "show", "interfaces"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output()?;
                return Ok(String::from_utf8_lossy(&output.stdout).replace('\0', ""));
            }
            Ok(None) if started.elapsed() >= NETSH_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(NetworkError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(NetworkError::Io(error)),
        }
    }
}

fn parse_netsh_wlan_signal(text: &str) -> HashMap<String, u8> {
    let mut signals = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_signal: Option<u8> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if let (Some(name), Some(signal)) = (current_name.take(), current_signal.take()) {
                signals.insert(normalize_adapter_key(&name), signal);
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if key == "name" {
            if let (Some(name), Some(signal)) = (current_name.take(), current_signal.take()) {
                signals.insert(normalize_adapter_key(&name), signal);
            }
            current_name = Some(value.to_owned());
        } else if key == "signal" {
            current_signal = value
                .trim_end_matches('%')
                .trim()
                .parse::<u8>()
                .ok()
                .map(|signal| signal.min(100));
        }
    }

    if let (Some(name), Some(signal)) = (current_name, current_signal) {
        signals.insert(normalize_adapter_key(&name), signal);
    }
    signals
}

fn vpn_state(adapters: &[NetworkAdapterInfo], processes: &[ProcessSnapshot]) -> VpnState {
    let mut evidence = Vec::new();
    let vpn_processes = processes
        .iter()
        .filter(|process| looks_like_vpn(&process.name))
        .take(12)
        .collect::<Vec<_>>();
    for process in &vpn_processes {
        evidence.push(NetworkEvidence {
            source: "process".to_owned(),
            detail: format!("VPN-like process is running: {}", process.name),
        });
    }
    let vpn_adapters = adapters
        .iter()
        .filter(|adapter| {
            adapter.operational_status == "up"
                && (looks_like_vpn(&adapter.name)
                    || adapter
                        .description
                        .as_ref()
                        .is_some_and(|description| looks_like_vpn(description))
                    || adapter.adapter_type == "tunnel")
        })
        .take(12)
        .collect::<Vec<_>>();
    for adapter in &vpn_adapters {
        evidence.push(NetworkEvidence {
            source: "adapter".to_owned(),
            detail: format!(
                "VPN/tunnel-like adapter is up: {} ({})",
                adapter.name, adapter.adapter_type
            ),
        });
    }

    let likely_active = !vpn_adapters.is_empty() || !vpn_processes.is_empty();
    let confidence = if !vpn_adapters.is_empty() && !vpn_processes.is_empty() {
        "strong"
    } else if likely_active {
        "inferred"
    } else {
        "none"
    }
    .to_owned();
    let label = if likely_active {
        format!("VPN likely active ({confidence} evidence)")
    } else {
        "No VPN evidence observed".to_owned()
    };

    VpnState {
        likely_active,
        confidence,
        label,
        evidence,
    }
}

fn looks_like_vpn(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "vpn",
        "wireguard",
        "tailscale",
        "openvpn",
        "nord",
        "proton",
        "expressvpn",
        "surfshark",
        "zerotier",
        "cloudflare warp",
        "cloudflared",
        "tunnel",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn local_dev_warnings(ports: &[PortEndpoint]) -> Vec<LocalDevServerWarning> {
    ports
        .iter()
        .filter(|endpoint| {
            endpoint.binding_scope == BindingScope::Loopback && endpoint.local_url.is_some()
        })
        .take(40)
        .map(|endpoint| LocalDevServerWarning {
            port: endpoint.local_port,
            address: endpoint.local_address.clone(),
            process_name: endpoint.owning_process_name.clone(),
            message: format!(
                "Port {} is bound only to loopback; another LAN device cannot use this binding directly.",
                endpoint.local_port
            ),
            remediation:
                "If you intentionally want LAN access, bind the dev server to 0.0.0.0 or a specific LAN interface and review firewall rules manually."
                    .to_owned(),
            lan_urls: endpoint.lan_urls.clone(),
        })
        .collect()
}

fn internet_latency_result() -> NetworkDiagnosticResult {
    let started = Instant::now();
    let state = match tcp_connect("example.com:80") {
        Ok(()) => NetworkDiagnosticState::Pass,
        Err(_) => NetworkDiagnosticState::Fail,
    };
    NetworkDiagnosticResult {
        label: "External latency".to_owned(),
        state,
        value: Some(format!(
            "{} ms TCP connect to example.com:80",
            started.elapsed().as_millis()
        )),
        local_only: false,
        contacted_internet: true,
        evidence: vec![NetworkEvidence {
            source: "tcp-connect".to_owned(),
            detail: "Explicit opt-in external TCP connection to example.com:80.".to_owned(),
        }],
    }
}

fn internet_dns_result() -> NetworkDiagnosticResult {
    match "example.com:80".to_socket_addrs() {
        Ok(addresses) => {
            let count = addresses.count();
            NetworkDiagnosticResult {
                label: "External DNS resolution".to_owned(),
                state: if count > 0 {
                    NetworkDiagnosticState::Pass
                } else {
                    NetworkDiagnosticState::Warn
                },
                value: Some(format!("{count} address record(s); values redacted")),
                local_only: false,
                contacted_internet: true,
                evidence: vec![NetworkEvidence {
                    source: "system-dns".to_owned(),
                    detail: "Resolved example.com; returned addresses are redacted.".to_owned(),
                }],
            }
        }
        Err(error) => NetworkDiagnosticResult {
            label: "External DNS resolution".to_owned(),
            state: NetworkDiagnosticState::Fail,
            value: None,
            local_only: false,
            contacted_internet: true,
            evidence: vec![NetworkEvidence {
                source: "system-dns".to_owned(),
                detail: redaction::redact(&error.to_string()),
            }],
        },
    }
}

fn packet_loss_result() -> NetworkDiagnosticResult {
    match ping_external_host("example.com") {
        Ok(true) => NetworkDiagnosticResult {
            label: "Packet loss probe".to_owned(),
            state: NetworkDiagnosticState::Pass,
            value: Some("ping.exe reported success for the external target".to_owned()),
            local_only: false,
            contacted_internet: true,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: "Ran exact opt-in external command: ping.exe -n 4 -w 1000 example.com"
                    .to_owned(),
            }],
        },
        Ok(false) => NetworkDiagnosticResult {
            label: "Packet loss probe".to_owned(),
            state: NetworkDiagnosticState::Warn,
            value: Some(
                "ping.exe did not report success; packet-loss percentage is not locale-parsed."
                    .to_owned(),
            ),
            local_only: false,
            contacted_internet: true,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: "External ICMP may be blocked by firewalls or VPN policy.".to_owned(),
            }],
        },
        Err(error) => NetworkDiagnosticResult {
            label: "Packet loss probe".to_owned(),
            state: NetworkDiagnosticState::Unavailable,
            value: None,
            local_only: false,
            contacted_internet: false,
            evidence: vec![NetworkEvidence {
                source: "ping.exe".to_owned(),
                detail: format!("Packet-loss probe could not run: {error}"),
            }],
        },
    }
}

fn download_probe_result() -> NetworkDiagnosticResult {
    let started = Instant::now();
    match http_get_limited("example.com:80", "/", SMALL_DOWNLOAD_LIMIT) {
        Ok(bytes) => {
            let elapsed = started.elapsed().as_secs_f64().max(0.001);
            let bytes_per_second = (bytes as f64 / elapsed) as u64;
            NetworkDiagnosticResult {
                label: "Small external download probe".to_owned(),
                state: NetworkDiagnosticState::Pass,
                value: Some(format!(
                    "{bytes} bytes received; approximate probe rate {bytes_per_second} B/s"
                )),
                local_only: false,
                contacted_internet: true,
                evidence: vec![NetworkEvidence {
                    source: "http-get".to_owned(),
                    detail:
                        "Downloaded a bounded response from example.com after explicit consent."
                            .to_owned(),
                }],
            }
        }
        Err(error) => NetworkDiagnosticResult {
            label: "Small external download probe".to_owned(),
            state: NetworkDiagnosticState::Fail,
            value: None,
            local_only: false,
            contacted_internet: true,
            evidence: vec![NetworkEvidence {
                source: "http-get".to_owned(),
                detail: redaction::redact(&error),
            }],
        },
    }
}

fn tcp_connect(address: &str) -> Result<(), String> {
    let mut addresses = address
        .to_socket_addrs()
        .map_err(|error| redaction::redact(&error.to_string()))?;
    let socket = addresses
        .next()
        .ok_or_else(|| "no address records returned".to_owned())?;
    TcpStream::connect_timeout(&socket, INTERNET_TIMEOUT)
        .map(|_| ())
        .map_err(|error| redaction::redact(&error.to_string()))
}

fn http_get_limited(address: &str, path: &str, max_bytes: usize) -> Result<usize, String> {
    let mut addresses = address
        .to_socket_addrs()
        .map_err(|error| redaction::redact(&error.to_string()))?;
    let socket = addresses
        .next()
        .ok_or_else(|| "no address records returned".to_owned())?;
    let mut stream =
        TcpStream::connect_timeout(&socket, INTERNET_TIMEOUT).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(INTERNET_TIMEOUT))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(INTERNET_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;
    let mut buffer = vec![0u8; max_bytes];
    let bytes = stream
        .take(max_bytes as u64)
        .read(&mut buffer)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn ping_host(host: &str) -> Result<bool, NetworkError> {
    run_ping(&["-n", "1", "-w", "1000", host], GATEWAY_PING_TIMEOUT)
}

fn ping_external_host(host: &str) -> Result<bool, NetworkError> {
    run_ping(
        &["-n", "4", "-w", "1000", host],
        Duration::from_millis(5_500),
    )
}

fn run_ping(args: &[&str], timeout: Duration) -> Result<bool, NetworkError> {
    let mut child = crate::platform::process::hidden_command("ping.exe")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.success()),
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(NetworkError::Timeout);
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => return Err(NetworkError::Io(error)),
        }
    }
}

fn normalize_adapter_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_lan_candidates_exclude_public_addresses() {
        assert_eq!(
            lan_candidate(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 25))),
            Some("192.168.1.25".to_owned())
        );
        assert_eq!(lan_candidate(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))), None);
        assert_eq!(
            safe_ip_display(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
            "redacted-public-ipv4"
        );
    }

    #[test]
    fn vpn_evidence_is_likely_not_certain() {
        assert!(looks_like_vpn("WireGuard Tunnel"));
        assert!(looks_like_vpn("tailscaled.exe"));
        assert!(!looks_like_vpn("Ethernet"));
    }

    #[test]
    fn adapter_history_tracks_session_peak_and_timeline() {
        let mut history = AdapterHistory::new(100, 50);
        let first = history.throughput(1, 10, 20, 110, 70);
        let second = history.throughput(2, 30, 5, 140, 80);
        assert_eq!(first.session_received_bytes, 10);
        assert_eq!(second.session_received_bytes, 40);
        assert_eq!(second.peak_received_bytes_per_second, 30);
        assert_eq!(second.timeline.len(), 2);
    }

    #[test]
    fn netsh_wifi_parser_keeps_signal_and_drops_ssid() {
        let signals = parse_netsh_wlan_signal(
            "Name                   : Wi-Fi\nDescription            : Adapter\nSSID                   : Secret Network\nSignal                 : 87%\n",
        );
        assert_eq!(signals.get("wi-fi"), Some(&87));
        assert_eq!(signals.len(), 1);
    }
}
