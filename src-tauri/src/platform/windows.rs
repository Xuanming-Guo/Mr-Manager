use std::{
    ffi::c_void,
    mem::{offset_of, size_of},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

use windows::Win32::{
    Foundation::ERROR_INSUFFICIENT_BUFFER,
    NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX, GET_ADAPTERS_ADDRESSES_FLAGS,
        GetAdaptersAddresses, GetExtendedTcpTable, GetExtendedUdpTable, IP_ADAPTER_ADDRESSES_LH,
        IP_ADAPTER_DNS_SERVER_ADDRESS_XP, IP_ADAPTER_GATEWAY_ADDRESS_LH,
        IP_ADAPTER_UNICAST_ADDRESS_LH, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
        MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID,
        MIB_UDP6TABLE_OWNER_PID, MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
        TCP_TABLE_OWNER_PID_LISTENER, UDP_TABLE_OWNER_PID,
    },
    NetworkManagement::Ndis::{
        IF_OPER_STATUS, IfOperStatusDormant, IfOperStatusDown, IfOperStatusLowerLayerDown,
        IfOperStatusNotPresent, IfOperStatusTesting, IfOperStatusUnknown, IfOperStatusUp,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
    Networking::WinSock::{AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS},
    System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
};
use windows::core::{PSTR, PWSTR};

use super::{NetworkAdapterInfo, OwnedPort, PlatformError, PowerStatus, TransportProtocol};

const NO_ERROR: u32 = 0;
const MAX_TABLE_BYTES: usize = 16 * 1024 * 1024;
const MAX_BUFFER_RETRIES: usize = 5;

const TCP_LISTEN_STATE: &str = "listen";
const UDP_BOUND_STATE: &str = "bound";

const BATTERY_FLAG_NO_SYSTEM_BATTERY: u8 = 128;
const BATTERY_FLAG_UNKNOWN: u8 = 255;
const AC_LINE_OFFLINE: u8 = 0;
const AC_LINE_ONLINE: u8 = 1;
const AC_LINE_UNKNOWN: u8 = 255;
const BATTERY_PERCENT_UNKNOWN: u8 = 255;
const BATTERY_LIFE_UNKNOWN: u32 = u32::MAX;

/// Word-backed storage gives the variable-length Windows tables at least pointer
/// alignment on every supported Windows target. The allocation is zero-filled so
/// checked reads never observe uninitialized bytes if an API returns bad metadata.
struct AlignedBuffer {
    words: Vec<usize>,
    byte_len: usize,
}

impl AlignedBuffer {
    fn allocate(operation: &'static str, byte_len: usize) -> Result<Self, PlatformError> {
        if byte_len == 0 {
            return Err(PlatformError::MalformedData {
                operation,
                reason: "the API requested a zero-length table buffer",
            });
        }
        if byte_len > MAX_TABLE_BYTES {
            return Err(PlatformError::BufferTooLarge {
                operation,
                requested: byte_len,
                limit: MAX_TABLE_BYTES,
            });
        }

        let word_size = size_of::<usize>();
        let word_len =
            byte_len
                .checked_add(word_size - 1)
                .ok_or(PlatformError::BufferTooLarge {
                    operation,
                    requested: byte_len,
                    limit: MAX_TABLE_BYTES,
                })?
                / word_size;
        let allocated_bytes =
            word_len
                .checked_mul(word_size)
                .ok_or(PlatformError::BufferTooLarge {
                    operation,
                    requested: byte_len,
                    limit: MAX_TABLE_BYTES,
                })?;

        let mut words = Vec::new();
        words
            .try_reserve_exact(word_len)
            .map_err(|_| PlatformError::AllocationFailed {
                operation,
                requested: allocated_bytes,
            })?;
        words.resize(word_len, 0);

        Ok(Self { words, byte_len })
    }

    fn as_ptr(&self) -> *const u8 {
        self.words.as_ptr().cast()
    }

    fn as_mut_void(&mut self) -> *mut c_void {
        self.words.as_mut_ptr().cast()
    }
}

pub(super) fn query_owned_ports() -> Result<Vec<OwnedPort>, PlatformError> {
    let mut ports = Vec::new();
    ports.extend(query_tcp_v4()?);
    ports.extend(query_tcp_v6()?);
    ports.extend(query_udp_v4()?);
    ports.extend(query_udp_v6()?);
    ports.sort_unstable();
    Ok(ports)
}

pub(super) fn query_power_status() -> Result<PowerStatus, PlatformError> {
    const OPERATION: &str = "GetSystemPowerStatus";

    let mut raw = SYSTEM_POWER_STATUS::default();
    // SAFETY: `raw` is a valid, writable SYSTEM_POWER_STATUS for the duration
    // of the call. The Windows API does not retain the pointer.
    unsafe { GetSystemPowerStatus(&mut raw) }.map_err(|error| PlatformError::WindowsApi {
        operation: OPERATION,
        code: error.code().0 as u32,
    })?;

    power_status_from_raw(&raw)
}

pub(super) fn query_network_adapters() -> Result<Vec<NetworkAdapterInfo>, PlatformError> {
    const OPERATION: &str = "GetAdaptersAddresses(AF_UNSPEC)";
    let buffer = query_variable_table(OPERATION, |table, size| {
        let flags =
            GET_ADAPTERS_ADDRESSES_FLAGS(GAA_FLAG_INCLUDE_GATEWAYS.0 | GAA_FLAG_INCLUDE_PREFIX.0);
        // SAFETY: `query_variable_table` supplies either None for the sizing
        // probe or a writable buffer of `*size` bytes. The API fills the linked
        // adapter records within that buffer and does not retain pointers.
        unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC.0),
                flags,
                None,
                table.map(|value| value.cast::<IP_ADAPTER_ADDRESSES_LH>()),
                size,
            )
        }
    })?;

    let mut adapters = Vec::new();
    let mut cursor = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    let mut guard = 0usize;
    while !cursor.is_null() {
        if guard > 4096 {
            return Err(PlatformError::MalformedData {
                operation: OPERATION,
                reason: "the adapter linked list did not terminate",
            });
        }
        // SAFETY: `cursor` points into the buffer returned by
        // GetAdaptersAddresses and is advanced only through documented `Next`
        // pointers while the owning buffer remains alive.
        let adapter = unsafe { &*cursor };
        let if_index = unsafe { adapter.Anonymous1.Anonymous.IfIndex };
        let name = non_empty(pwstr_to_string(adapter.FriendlyName))
            .or_else(|| non_empty(pstr_to_string(adapter.AdapterName)))
            .unwrap_or_else(|| format!("Interface {if_index}"));
        let description = non_empty(pwstr_to_string(adapter.Description));
        let (ipv4_addresses, ipv6_addresses) =
            collect_unicast_addresses(adapter.FirstUnicastAddress);
        adapters.push(NetworkAdapterInfo {
            id: if adapter.NetworkGuid != Default::default() {
                format!("{:?}", adapter.NetworkGuid)
            } else {
                format!("if-{if_index}-{}", adapter.Ipv6IfIndex)
            },
            name,
            description,
            adapter_type: if_type_label(adapter.IfType).to_owned(),
            operational_status: oper_status_label(adapter.OperStatus).to_owned(),
            ipv4_addresses,
            ipv6_addresses,
            gateway_addresses: collect_gateway_addresses(adapter.FirstGatewayAddress),
            dns_server_count: count_dns_servers(adapter.FirstDnsServerAddress),
            receive_link_speed_bits_per_second: non_zero(adapter.ReceiveLinkSpeed),
            transmit_link_speed_bits_per_second: non_zero(adapter.TransmitLinkSpeed),
            ipv4_metric: non_zero_u32(adapter.Ipv4Metric),
            ipv6_metric: non_zero_u32(adapter.Ipv6Metric),
        });
        cursor = adapter.Next;
        guard += 1;
    }

    adapters.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(adapters)
}

fn query_tcp_v4() -> Result<Vec<OwnedPort>, PlatformError> {
    const OPERATION: &str = "GetExtendedTcpTable(AF_INET)";
    let buffer = query_variable_table(OPERATION, |table, size| {
        // SAFETY: `query_variable_table` supplies either None for the sizing
        // probe or a valid writable buffer of `*size` bytes. Other arguments
        // select the documented PID-owning listener table.
        unsafe {
            GetExtendedTcpTable(
                table,
                size,
                false,
                u32::from(AF_INET.0),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        }
    })?;
    let rows = read_rows::<MIB_TCPROW_OWNER_PID>(
        &buffer,
        offset_of!(MIB_TCPTABLE_OWNER_PID, table),
        OPERATION,
    )?;

    Ok(rows.into_iter().map(tcp_v4_port).collect())
}

fn collect_unicast_addresses(
    mut cursor: *mut IP_ADAPTER_UNICAST_ADDRESS_LH,
) -> (Vec<IpAddr>, Vec<IpAddr>) {
    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();
    let mut guard = 0usize;
    while !cursor.is_null() && guard < 256 {
        // SAFETY: The address node belongs to the GetAdaptersAddresses buffer.
        let node = unsafe { &*cursor };
        if let Some(address) = socket_address_to_ip(&node.Address) {
            match address {
                IpAddr::V4(_) => ipv4.push(address),
                IpAddr::V6(_) => ipv6.push(address),
            }
        }
        cursor = node.Next;
        guard += 1;
    }
    ipv4.sort();
    ipv4.dedup();
    ipv6.sort();
    ipv6.dedup();
    (ipv4, ipv6)
}

fn collect_gateway_addresses(mut cursor: *mut IP_ADAPTER_GATEWAY_ADDRESS_LH) -> Vec<IpAddr> {
    let mut gateways = Vec::new();
    let mut guard = 0usize;
    while !cursor.is_null() && guard < 64 {
        // SAFETY: The address node belongs to the GetAdaptersAddresses buffer.
        let node = unsafe { &*cursor };
        if let Some(address) = socket_address_to_ip(&node.Address) {
            gateways.push(address);
        }
        cursor = node.Next;
        guard += 1;
    }
    gateways.sort();
    gateways.dedup();
    gateways
}

fn count_dns_servers(mut cursor: *mut IP_ADAPTER_DNS_SERVER_ADDRESS_XP) -> u32 {
    let mut count = 0u32;
    let mut guard = 0usize;
    while !cursor.is_null() && guard < 64 {
        // SAFETY: The address node belongs to the GetAdaptersAddresses buffer.
        let node = unsafe { &*cursor };
        count = count.saturating_add(u32::from(socket_address_to_ip(&node.Address).is_some()));
        cursor = node.Next;
        guard += 1;
    }
    count
}

fn socket_address_to_ip(address: &SOCKET_ADDRESS) -> Option<IpAddr> {
    let raw = address.lpSockaddr;
    if raw.is_null() {
        return None;
    }
    // SAFETY: `raw` is supplied by Windows as a SOCKADDR with at least the
    // family field. The concrete casts are gated by the family and expected
    // structure size from the API.
    let family = unsafe { (*(raw as *const SOCKADDR)).sa_family };
    if family == AF_INET && address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN>() {
        // SAFETY: Family and length indicate an IPv4 socket address.
        let socket = unsafe { &*(raw as *const SOCKADDR_IN) };
        let octets = unsafe { socket.sin_addr.S_un.S_addr.to_ne_bytes() };
        Some(IpAddr::V4(Ipv4Addr::from(octets)))
    } else if family == AF_INET6 && address.iSockaddrLength as usize >= size_of::<SOCKADDR_IN6>() {
        // SAFETY: Family and length indicate an IPv6 socket address.
        let socket = unsafe { &*(raw as *const SOCKADDR_IN6) };
        let octets = unsafe { socket.sin6_addr.u.Byte };
        Some(IpAddr::V6(Ipv6Addr::from(octets)))
    } else {
        None
    }
}

fn pstr_to_string(value: PSTR) -> String {
    // SAFETY: Windows owns these null-terminated strings for the duration of
    // the adapter buffer. Invalid UTF-8 is converted lossily by the crate.
    unsafe { value.to_string().unwrap_or_default() }
}

fn pwstr_to_string(value: PWSTR) -> String {
    // SAFETY: Windows owns these null-terminated strings for the duration of
    // the adapter buffer.
    unsafe { value.to_string().unwrap_or_default() }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn non_zero(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

fn non_zero_u32(value: u32) -> Option<u32> {
    (value > 0).then_some(value)
}

fn if_type_label(if_type: u32) -> &'static str {
    match if_type {
        6 => "ethernet",
        9 => "token-ring",
        23 => "ppp",
        24 => "loopback",
        37 => "atm",
        53 => "prop-virtual",
        71 => "wi-fi",
        131 => "tunnel",
        243 => "wwan",
        244 => "wwan",
        _ => "other",
    }
}

fn oper_status_label(status: IF_OPER_STATUS) -> &'static str {
    match status.0 {
        value if value == IfOperStatusUp.0 => "up",
        value if value == IfOperStatusDown.0 => "down",
        value if value == IfOperStatusTesting.0 => "testing",
        value if value == IfOperStatusUnknown.0 => "unknown",
        value if value == IfOperStatusDormant.0 => "dormant",
        value if value == IfOperStatusNotPresent.0 => "not-present",
        value if value == IfOperStatusLowerLayerDown.0 => "lower-layer-down",
        _ => "unknown",
    }
}

fn query_tcp_v6() -> Result<Vec<OwnedPort>, PlatformError> {
    const OPERATION: &str = "GetExtendedTcpTable(AF_INET6)";
    let buffer = query_variable_table(OPERATION, |table, size| {
        // SAFETY: See `query_tcp_v4`; this call selects the IPv6 table.
        unsafe {
            GetExtendedTcpTable(
                table,
                size,
                false,
                u32::from(AF_INET6.0),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        }
    })?;
    let rows = read_rows::<MIB_TCP6ROW_OWNER_PID>(
        &buffer,
        offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
        OPERATION,
    )?;

    Ok(rows.into_iter().map(tcp_v6_port).collect())
}

fn query_udp_v4() -> Result<Vec<OwnedPort>, PlatformError> {
    const OPERATION: &str = "GetExtendedUdpTable(AF_INET)";
    let buffer = query_variable_table(OPERATION, |table, size| {
        // SAFETY: `query_variable_table` owns the correctly sized writable
        // buffer. Other arguments select the documented PID-owning UDP table.
        unsafe {
            GetExtendedUdpTable(
                table,
                size,
                false,
                u32::from(AF_INET.0),
                UDP_TABLE_OWNER_PID,
                0,
            )
        }
    })?;
    let rows = read_rows::<MIB_UDPROW_OWNER_PID>(
        &buffer,
        offset_of!(MIB_UDPTABLE_OWNER_PID, table),
        OPERATION,
    )?;

    Ok(rows.into_iter().map(udp_v4_port).collect())
}

fn query_udp_v6() -> Result<Vec<OwnedPort>, PlatformError> {
    const OPERATION: &str = "GetExtendedUdpTable(AF_INET6)";
    let buffer = query_variable_table(OPERATION, |table, size| {
        // SAFETY: See `query_udp_v4`; this call selects the IPv6 table.
        unsafe {
            GetExtendedUdpTable(
                table,
                size,
                false,
                u32::from(AF_INET6.0),
                UDP_TABLE_OWNER_PID,
                0,
            )
        }
    })?;
    let rows = read_rows::<MIB_UDP6ROW_OWNER_PID>(
        &buffer,
        offset_of!(MIB_UDP6TABLE_OWNER_PID, table),
        OPERATION,
    )?;

    Ok(rows.into_iter().map(udp_v6_port).collect())
}

fn query_variable_table(
    operation: &'static str,
    mut call: impl FnMut(Option<*mut c_void>, *mut u32) -> u32,
) -> Result<AlignedBuffer, PlatformError> {
    let mut requested = 0_u32;
    let probe_status = call(None, &mut requested);
    if probe_status != NO_ERROR && probe_status != ERROR_INSUFFICIENT_BUFFER.0 {
        return Err(PlatformError::WindowsApi {
            operation,
            code: probe_status,
        });
    }

    for _ in 0..MAX_BUFFER_RETRIES {
        let requested_usize =
            usize::try_from(requested).map_err(|_| PlatformError::BufferTooLarge {
                operation,
                requested: usize::MAX,
                limit: MAX_TABLE_BYTES,
            })?;
        let mut buffer = AlignedBuffer::allocate(operation, requested_usize)?;
        let mut supplied_size = requested;
        let status = call(Some(buffer.as_mut_void()), &mut supplied_size);

        if status == NO_ERROR {
            if supplied_size != 0 {
                buffer.byte_len = buffer.byte_len.min(supplied_size as usize);
            }
            return Ok(buffer);
        }
        if status != ERROR_INSUFFICIENT_BUFFER.0 {
            return Err(PlatformError::WindowsApi {
                operation,
                code: status,
            });
        }
        requested = supplied_size;
    }

    Err(PlatformError::BufferUnstable {
        operation,
        attempts: MAX_BUFFER_RETRIES,
    })
}

fn read_rows<Row: Copy>(
    buffer: &AlignedBuffer,
    row_offset: usize,
    operation: &'static str,
) -> Result<Vec<Row>, PlatformError> {
    if buffer.byte_len < size_of::<u32>() {
        return Err(PlatformError::MalformedData {
            operation,
            reason: "the table is smaller than its entry-count header",
        });
    }

    // SAFETY: The four-byte header was bounds-checked above. `read_unaligned`
    // avoids relying on the Windows table header's alignment.
    let count = unsafe { ptr::read_unaligned(buffer.as_ptr().cast::<u32>()) } as usize;
    if count == 0 {
        return Ok(Vec::new());
    }

    let rows_len = count
        .checked_mul(size_of::<Row>())
        .ok_or(PlatformError::MalformedData {
            operation,
            reason: "the table entry count overflows addressable memory",
        })?;
    let rows_end = row_offset
        .checked_add(rows_len)
        .ok_or(PlatformError::MalformedData {
            operation,
            reason: "the table row range overflows addressable memory",
        })?;
    if row_offset < size_of::<u32>() || rows_end > buffer.byte_len {
        return Err(PlatformError::MalformedData {
            operation,
            reason: "the entry count exceeds the returned table buffer",
        });
    }

    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| PlatformError::AllocationFailed {
            operation,
            requested: rows_len,
        })?;

    for index in 0..count {
        let byte_offset = row_offset + index * size_of::<Row>();
        // SAFETY: The complete row range was checked above. Windows row types
        // are Copy POD structures, and unaligned reads also tolerate any table
        // padding chosen by the SDK ABI.
        let row = unsafe { ptr::read_unaligned(buffer.as_ptr().add(byte_offset).cast::<Row>()) };
        rows.push(row);
    }
    Ok(rows)
}

fn tcp_v4_port(row: MIB_TCPROW_OWNER_PID) -> OwnedPort {
    OwnedPort {
        protocol: TransportProtocol::Tcp,
        local_address: IpAddr::V4(ipv4_from_network_dword(row.dwLocalAddr)),
        scope_id: 0,
        local_port: port_from_network_dword(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
        state: TCP_LISTEN_STATE,
    }
}

fn tcp_v6_port(row: MIB_TCP6ROW_OWNER_PID) -> OwnedPort {
    OwnedPort {
        protocol: TransportProtocol::Tcp,
        local_address: IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
        scope_id: u32_from_network_dword(row.dwLocalScopeId),
        local_port: port_from_network_dword(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
        state: TCP_LISTEN_STATE,
    }
}

fn udp_v4_port(row: MIB_UDPROW_OWNER_PID) -> OwnedPort {
    OwnedPort {
        protocol: TransportProtocol::Udp,
        local_address: IpAddr::V4(ipv4_from_network_dword(row.dwLocalAddr)),
        scope_id: 0,
        local_port: port_from_network_dword(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
        state: UDP_BOUND_STATE,
    }
}

fn udp_v6_port(row: MIB_UDP6ROW_OWNER_PID) -> OwnedPort {
    OwnedPort {
        protocol: TransportProtocol::Udp,
        local_address: IpAddr::V6(Ipv6Addr::from(row.ucLocalAddr)),
        scope_id: u32_from_network_dword(row.dwLocalScopeId),
        local_port: port_from_network_dword(row.dwLocalPort),
        owning_pid: row.dwOwningPid,
        state: UDP_BOUND_STATE,
    }
}

fn ipv4_from_network_dword(value: u32) -> Ipv4Addr {
    Ipv4Addr::from(value.to_ne_bytes())
}

fn port_from_network_dword(value: u32) -> u16 {
    let bytes = value.to_ne_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn u32_from_network_dword(value: u32) -> u32 {
    u32::from_be_bytes(value.to_ne_bytes())
}

fn power_status_from_raw(raw: &SYSTEM_POWER_STATUS) -> Result<PowerStatus, PlatformError> {
    const OPERATION: &str = "GetSystemPowerStatus";

    let battery_present = match raw.BatteryFlag {
        BATTERY_FLAG_UNKNOWN => {
            return Err(PlatformError::MalformedData {
                operation: OPERATION,
                reason: "Windows reported unknown battery presence",
            });
        }
        flag => flag & BATTERY_FLAG_NO_SYSTEM_BATTERY == 0,
    };
    let ac_online = match raw.ACLineStatus {
        AC_LINE_OFFLINE => Some(false),
        AC_LINE_ONLINE => Some(true),
        AC_LINE_UNKNOWN => None,
        _ => {
            return Err(PlatformError::MalformedData {
                operation: OPERATION,
                reason: "Windows returned an invalid AC-line state",
            });
        }
    };
    let battery_percent = if !battery_present || raw.BatteryLifePercent == BATTERY_PERCENT_UNKNOWN {
        None
    } else if raw.BatteryLifePercent <= 100 {
        Some(raw.BatteryLifePercent)
    } else {
        return Err(PlatformError::MalformedData {
            operation: OPERATION,
            reason: "Windows returned an invalid battery percentage",
        });
    };
    let remaining_seconds = if !battery_present || raw.BatteryLifeTime == BATTERY_LIFE_UNKNOWN {
        None
    } else {
        Some(raw.BatteryLifeTime)
    };

    Ok(PowerStatus {
        battery_present,
        ac_online,
        battery_percent,
        remaining_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_OPERATION: &str = "synthetic table";

    #[test]
    fn aligned_buffer_meets_all_windows_row_alignments() {
        let buffer = AlignedBuffer::allocate(TEST_OPERATION, 64).expect("allocation should work");
        let address = buffer.as_ptr() as usize;

        assert_eq!(address % std::mem::align_of::<MIB_TCPROW_OWNER_PID>(), 0);
        assert_eq!(address % std::mem::align_of::<MIB_TCP6ROW_OWNER_PID>(), 0);
        assert_eq!(address % std::mem::align_of::<MIB_UDPROW_OWNER_PID>(), 0);
        assert_eq!(address % std::mem::align_of::<MIB_UDP6ROW_OWNER_PID>(), 0);
    }

    #[test]
    fn network_order_helpers_decode_api_memory_layout() {
        let ipv4_raw = u32::from_ne_bytes([127, 0, 0, 1]);
        let port_raw = u32::from_ne_bytes([0x1f, 0x90, 0, 0]);
        let scope_raw = u32::from_ne_bytes([0, 0, 0, 7]);

        assert_eq!(
            ipv4_from_network_dword(ipv4_raw),
            Ipv4Addr::new(127, 0, 0, 1)
        );
        assert_eq!(port_from_network_dword(port_raw), 8080);
        assert_eq!(u32_from_network_dword(scope_raw), 7);
    }

    #[test]
    fn reads_checked_rows_from_a_synthetic_variable_table() {
        let row_offset = offset_of!(MIB_UDPTABLE_OWNER_PID, table);
        let required = row_offset + size_of::<MIB_UDPROW_OWNER_PID>();
        let mut buffer =
            AlignedBuffer::allocate(TEST_OPERATION, required).expect("allocation should work");
        let row = MIB_UDPROW_OWNER_PID {
            dwLocalAddr: u32::from_ne_bytes([127, 0, 0, 1]),
            dwLocalPort: u32::from_ne_bytes([0x14, 0x46, 0, 0]),
            dwOwningPid: 42,
        };

        // SAFETY: The synthetic allocation was sized for exactly this header
        // and row layout, and both writes remain within its bounds.
        unsafe {
            ptr::write_unaligned(buffer.as_mut_void().cast::<u32>(), 1);
            ptr::write_unaligned(
                buffer
                    .as_mut_void()
                    .cast::<u8>()
                    .add(row_offset)
                    .cast::<MIB_UDPROW_OWNER_PID>(),
                row,
            );
        }

        let rows = read_rows::<MIB_UDPROW_OWNER_PID>(&buffer, row_offset, TEST_OPERATION)
            .expect("well-formed table should parse");
        assert_eq!(rows, vec![row]);
        let endpoint = udp_v4_port(rows[0]);
        assert_eq!(endpoint.local_address, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(endpoint.local_port, 5190);
        assert_eq!(endpoint.owning_pid, 42);
        assert_eq!(endpoint.state, UDP_BOUND_STATE);
    }

    #[test]
    fn rejects_a_count_that_exceeds_the_table_buffer() {
        let row_offset = offset_of!(MIB_UDPTABLE_OWNER_PID, table);
        let mut buffer =
            AlignedBuffer::allocate(TEST_OPERATION, row_offset).expect("allocation should work");

        // SAFETY: Every Windows owner table begins with this four-byte count,
        // and the allocation is at least that large.
        unsafe { ptr::write_unaligned(buffer.as_mut_void().cast::<u32>(), 1) };

        assert!(matches!(
            read_rows::<MIB_UDPROW_OWNER_PID>(&buffer, row_offset, TEST_OPERATION),
            Err(PlatformError::MalformedData {
                operation: TEST_OPERATION,
                reason: "the entry count exceeds the returned table buffer",
            })
        ));
    }

    #[test]
    fn maps_power_states_without_inventing_unknown_ac_status() {
        let on_ac = SYSTEM_POWER_STATUS {
            ACLineStatus: AC_LINE_ONLINE,
            BatteryFlag: 8,
            BatteryLifePercent: 75,
            BatteryLifeTime: 3_600,
            ..Default::default()
        };
        let desktop = SYSTEM_POWER_STATUS {
            ACLineStatus: AC_LINE_UNKNOWN,
            BatteryFlag: BATTERY_FLAG_NO_SYSTEM_BATTERY,
            ..Default::default()
        };

        assert_eq!(
            power_status_from_raw(&on_ac).expect("valid power status"),
            PowerStatus {
                battery_present: true,
                ac_online: Some(true),
                battery_percent: Some(75),
                remaining_seconds: Some(3_600),
            }
        );
        assert_eq!(
            power_status_from_raw(&desktop).expect("valid desktop status"),
            PowerStatus {
                battery_present: false,
                ac_online: None,
                battery_percent: None,
                remaining_seconds: None,
            }
        );
    }

    #[test]
    fn rejects_unknown_battery_presence() {
        let raw = SYSTEM_POWER_STATUS {
            ACLineStatus: AC_LINE_ONLINE,
            BatteryFlag: BATTERY_FLAG_UNKNOWN,
            ..Default::default()
        };

        assert!(matches!(
            power_status_from_raw(&raw),
            Err(PlatformError::MalformedData {
                operation: "GetSystemPowerStatus",
                reason: "Windows reported unknown battery presence",
            })
        ));
    }
}
