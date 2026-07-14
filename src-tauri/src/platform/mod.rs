//! Narrow operating-system adapters used by the backend collectors.

use std::net::IpAddr;

use thiserror::Error;

pub mod process;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod windows_impl;

/// Transport represented by an operating-system-owned local endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

/// A local TCP listener or bound UDP endpoint and its owning process.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnedPort {
    pub protocol: TransportProtocol,
    pub local_address: IpAddr,
    pub scope_id: u32,
    pub local_port: u16,
    pub owning_pid: u32,
    pub state: &'static str,
}

/// Power information that is reliably available from every supported adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerStatus {
    pub battery_present: bool,
    pub ac_online: Option<bool>,
    pub battery_percent: Option<u8>,
    pub remaining_seconds: Option<u32>,
}

/// Local adapter metadata from the operating system. Sensitive fields such as
/// MAC addresses, SSIDs, public IPs, and DNS server addresses are deliberately
/// not returned by this adapter boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAdapterInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub adapter_type: String,
    pub operational_status: String,
    pub ipv4_addresses: Vec<IpAddr>,
    pub ipv6_addresses: Vec<IpAddr>,
    pub gateway_addresses: Vec<IpAddr>,
    pub dns_server_count: u32,
    pub receive_link_speed_bits_per_second: Option<u64>,
    pub transmit_link_speed_bits_per_second: Option<u64>,
    pub ipv4_metric: Option<u32>,
    pub ipv6_metric: Option<u32>,
}

/// A typed failure from a platform adapter.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlatformError {
    #[allow(dead_code)] // Constructed by non-Windows adapters; Windows is the first target.
    #[error("{operation} is unsupported on {platform}")]
    Unsupported {
        operation: &'static str,
        platform: &'static str,
    },

    #[error("{operation} failed with Windows error code {code}")]
    WindowsApi { operation: &'static str, code: u32 },

    #[error("{operation} returned malformed data: {reason}")]
    MalformedData {
        operation: &'static str,
        reason: &'static str,
    },

    #[error("{operation} requested a {requested}-byte buffer, exceeding the {limit}-byte limit")]
    BufferTooLarge {
        operation: &'static str,
        requested: usize,
        limit: usize,
    },

    #[error("unable to allocate {requested} bytes for {operation}")]
    AllocationFailed {
        operation: &'static str,
        requested: usize,
    },

    #[error("{operation} did not stabilize after {attempts} buffer-size retries")]
    BufferUnstable {
        operation: &'static str,
        attempts: usize,
    },
}

/// Returns local network adapter metadata without contacting external hosts.
pub fn query_network_adapters() -> Result<Vec<NetworkAdapterInfo>, PlatformError> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::query_network_adapters()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(PlatformError::Unsupported {
            operation: "network adapter enumeration",
            platform: std::env::consts::OS,
        })
    }
}

/// Returns TCP listeners and bound UDP endpoints with owning PIDs.
pub fn query_owned_ports() -> Result<Vec<OwnedPort>, PlatformError> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::query_owned_ports()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(PlatformError::Unsupported {
            operation: "owned port enumeration",
            platform: std::env::consts::OS,
        })
    }
}

/// Returns basic battery-presence and AC-line information.
pub fn query_power_status() -> Result<PowerStatus, PlatformError> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::query_power_status()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(PlatformError::Unsupported {
            operation: "power status query",
            platform: std::env::consts::OS,
        })
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_platform_returns_typed_errors() {
        assert!(matches!(
            query_owned_ports(),
            Err(PlatformError::Unsupported {
                operation: "owned port enumeration",
                platform: std::env::consts::OS,
            })
        ));
        assert!(matches!(
            query_power_status(),
            Err(PlatformError::Unsupported {
                operation: "power status query",
                platform: std::env::consts::OS,
            })
        ));
    }
}
