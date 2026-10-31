use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::mem::zeroed;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ptr::null_mut;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::*;
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows_sys::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows_sys::Win32::System::Threading::{
    GetProcessIoCounters, OpenProcess, IO_COUNTERS, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAdapterInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub adapter_type: String,
    pub is_up: bool,
    pub speed_bps: u64,
    pub in_octets: u64,
    pub out_octets: u64,
    pub ip_addresses: Vec<String>,
    pub mac_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SocketEndpoint {
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
    pub ip_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTcpConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
    pub ip_version: String,
    pub owner_pid: u32,
    pub process_name: String,
    pub process_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSocketConnection {
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
    pub ip_version: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNetworkReport {
    pub pid: u32,
    pub name: String,
    pub path: String,
    #[serde(rename = "activeConnections")]
    pub active_connections: u32,
    pub connections: Vec<ProcessSocketConnection>,
    #[serde(rename = "ioReadTransferBytes")]
    pub io_read_transfer_bytes: u64,
    #[serde(rename = "ioWriteTransferBytes")]
    pub io_write_transfer_bytes: u64,
    #[serde(rename = "ioReadRateBps")]
    pub io_read_rate_bps: f64,
    #[serde(rename = "ioWriteRateBps")]
    pub io_write_rate_bps: f64,
    #[serde(rename = "isStreaming")]
    pub is_streaming: bool,
    #[serde(rename = "streamingConfidence")]
    pub streaming_confidence: String,
    #[serde(rename = "streamingReason")]
    pub streaming_reason: Option<String>,
    #[serde(rename = "bandwidthMeasurementType")]
    pub bandwidth_measurement_type: String,
}

/// Retrieve all hardware network interfaces using Windows GetIfTable2
pub fn query_interface_table() -> Vec<RawAdapterInfo> {
    let mut adapters = Vec::new();

    unsafe {
        let mut table_ptr: *mut MIB_IF_TABLE2 = null_mut();
        let res = GetIfTable2(&mut table_ptr);

        if res == NO_ERROR && !table_ptr.is_null() {
            let table = &*table_ptr;
            let num_entries = table.NumEntries as usize;
            let slice = std::slice::from_raw_parts(table.Table.as_ptr(), num_entries);

            for row in slice {
                // IfOperStatusUp = 1
                let is_up = row.OperStatus == 1;
                let adapter_type = match row.Type {
                    6 => "ethernet",
                    71 => "wifi",
                    23 | 243 => "cellular",
                    24 => "loopback",
                    _ => "other",
                };

                let desc_len = row.Description.iter().position(|&c| c == 0).unwrap_or(row.Description.len());
                let description = String::from_utf16_lossy(&row.Description[..desc_len]);

                let alias_len = row.Alias.iter().position(|&c| c == 0).unwrap_or(row.Alias.len());
                let name = if alias_len > 0 {
                    String::from_utf16_lossy(&row.Alias[..alias_len])
                } else {
                    description.clone()
                };

                let mac = if row.PhysicalAddressLength > 0 {
                    let len = (row.PhysicalAddressLength as usize).min(row.PhysicalAddress.len());
                    row.PhysicalAddress[..len]
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    "N/A".to_string()
                };

                adapters.push(RawAdapterInfo {
                    id: format!("if-{}", row.InterfaceIndex),
                    name,
                    description,
                    adapter_type: adapter_type.to_string(),
                    is_up,
                    speed_bps: row.ReceiveLinkSpeed,
                    in_octets: row.InOctets,
                    out_octets: row.OutOctets,
                    ip_addresses: Vec::new(),
                    mac_address: mac,
                });
            }

            FreeMibTable(table_ptr as *const c_void);
        }
    }

    adapters
}

/// Query active TCP sockets (IPv4 + IPv6) mapped to owner PIDs
pub fn query_extended_tcp_table() -> Vec<RawTcpConnection> {
    let mut connections = Vec::new();

    // 1. Query IPv4 TCP Table
    unsafe {
        let mut size = 65536u32;
        let mut buffer: Vec<u8> = vec![0; size as usize];

        let mut res = GetExtendedTcpTable(
            buffer.as_mut_ptr() as *mut c_void,
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );

        if res == 122 {
            buffer.resize(size as usize, 0);
            res = GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut c_void,
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if res == NO_ERROR {
            let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let num_entries = table.dwNumEntries as usize;
            let rows_ptr = table.table.as_ptr();
            let slice = std::slice::from_raw_parts(rows_ptr, num_entries);

            for row in slice {
                let local_ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr));
                let local_port = u16::from_be(row.dwLocalPort as u16);
                let remote_ip = Ipv4Addr::from(u32::from_be(row.dwRemoteAddr));
                let remote_port = u16::from_be(row.dwRemotePort as u16);

                let state = tcp_state_to_str(row.dwState);
                let (proc_name, proc_path) = get_process_info_by_pid(row.dwOwningPid);

                connections.push(RawTcpConnection {
                    local_addr: format!("{}:{}", local_ip, local_port),
                    remote_addr: format!("{}:{}", remote_ip, remote_port),
                    state: state.to_string(),
                    ip_version: "IPv4".to_string(),
                    owner_pid: row.dwOwningPid,
                    process_name: proc_name,
                    process_path: proc_path,
                });
            }
        }
    }

    // 2. Query IPv6 TCP Table
    unsafe {
        let mut size = 65536u32;
        let mut buffer: Vec<u8> = vec![0; size as usize];

        let mut res = GetExtendedTcpTable(
            buffer.as_mut_ptr() as *mut c_void,
            &mut size,
            0,
            AF_INET6 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );

        if res == 122 {
            buffer.resize(size as usize, 0);
            res = GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut c_void,
                &mut size,
                0,
                AF_INET6 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if res == NO_ERROR {
            let table = &*(buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
            let num_entries = table.dwNumEntries as usize;
            let rows_ptr = table.table.as_ptr();
            let slice = std::slice::from_raw_parts(rows_ptr, num_entries);

            for row in slice {
                let local_ip = Ipv6Addr::from(row.ucLocalAddr);
                let local_port = u16::from_be(row.dwLocalPort as u16);
                let remote_ip = Ipv6Addr::from(row.ucRemoteAddr);
                let remote_port = u16::from_be(row.dwRemotePort as u16);

                let state = tcp_state_to_str(row.dwState);
                let (proc_name, proc_path) = get_process_info_by_pid(row.dwOwningPid);

                connections.push(RawTcpConnection {
                    local_addr: format!("[{}]:{}", local_ip, local_port),
                    remote_addr: format!("[{}]:{}", remote_ip, remote_port),
                    state: state.to_string(),
                    ip_version: "IPv6".to_string(),
                    owner_pid: row.dwOwningPid,
                    process_name: proc_name,
                    process_path: proc_path,
                });
            }
        }
    }

    connections
}

fn tcp_state_to_str(state_id: u32) -> &'static str {
    match state_id {
        1 => "CLOSED",
        2 => "LISTEN",
        3 => "SYN_SENT",
        4 => "SYN_RCVD",
        5 => "ESTABLISHED",
        6 => "FIN_WAIT1",
        7 => "FIN_WAIT2",
        8 => "CLOSE_WAIT",
        9 => "CLOSING",
        10 => "LAST_ACK",
        11 => "TIME_WAIT",
        12 => "DELETE_TCB",
        _ => "UNKNOWN",
    }
}

/// Resolves process name and full executable path
pub fn get_process_info_by_pid(pid: u32) -> (String, String) {
    if pid == 0 {
        return ("System Idle Process".to_string(), "".to_string());
    }
    if pid == 4 {
        return ("System".to_string(), "C:\\Windows\\System32\\ntoskrnl.exe".to_string());
    }

    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            0,
            pid,
        );

        if !handle.is_null() {
            let mut name_buf = [0u16; 260];
            let len = GetModuleBaseNameW(handle, null_mut(), name_buf.as_mut_ptr(), name_buf.len() as u32);
            let name = if len > 0 {
                String::from_utf16_lossy(&name_buf[..len as usize])
            } else {
                format!("Process ({})", pid)
            };

            let mut path_buf = [0u16; 1024];
            let mut size = path_buf.len() as u32;
            let success = windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
                handle,
                0,
                path_buf.as_mut_ptr(),
                &mut size,
            );
            CloseHandle(handle);

            let path = if success != 0 && size > 0 {
                String::from_utf16_lossy(&path_buf[..size as usize])
            } else {
                "".to_string()
            };

            return (name, path);
        }

        // Fallback with limited information handle
        let limited_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !limited_handle.is_null() {
            let mut path_buf = [0u16; 1024];
            let mut size = path_buf.len() as u32;
            let success = windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
                limited_handle,
                0,
                path_buf.as_mut_ptr(),
                &mut size,
            );
            CloseHandle(limited_handle);

            if success != 0 && size > 0 {
                let full_path = String::from_utf16_lossy(&path_buf[..size as usize]);
                let name = if let Some(pos) = full_path.rfind('\\') {
                    full_path[pos + 1..].to_string()
                } else {
                    full_path.clone()
                };
                return (name, full_path);
            }
        }
    }

    (format!("Process ({})", pid), "".to_string())
}

/// Query actual I/O transfer counters for a process using Win32 GetProcessIoCounters
pub fn query_process_io_counters(pid: u32) -> Option<(u64, u64)> {
    if pid <= 4 {
        return None;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_QUERY_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut io: IO_COUNTERS = zeroed();
        let res = GetProcessIoCounters(handle, &mut io);
        CloseHandle(handle);

        if res != 0 {
            Some((io.ReadTransferCount, io.WriteTransferCount))
        } else {
            None
        }
    }
}

/// Tracks connection state transitions and process I/O rate over time
pub struct ProcessNetworkMonitor {
    previous_connections: HashSet<(u32, String, String)>, // (pid, local, remote)
    previous_io_samples: HashMap<u32, (Instant, u64, u64)>, // pid -> (time, read_bytes, write_bytes)
}

impl ProcessNetworkMonitor {
    pub fn new() -> Self {
        Self {
            previous_connections: HashSet::new(),
            previous_io_samples: HashMap::new(),
        }
    }

    pub fn snapshot_processes(&mut self) -> Vec<ProcessNetworkReport> {
        self.snapshot_processes_with_detector(None)
    }

    pub fn snapshot_processes_with_detector(
        &mut self,
        detector: Option<&crate::streaming::detector::StreamingDetector>,
    ) -> Vec<ProcessNetworkReport> {
        let raw_connections = query_extended_tcp_table();
        let now = Instant::now();

        let mut current_conns_set = HashSet::new();
        let mut grouped: HashMap<u32, (String, String, Vec<ProcessSocketConnection>)> = HashMap::new();

        for conn in &raw_connections {
            let key = (conn.owner_pid, conn.local_addr.clone(), conn.remote_addr.clone());
            let is_new = !self.previous_connections.is_empty() && !self.previous_connections.contains(&key);
            current_conns_set.insert(key);

            let entry = grouped.entry(conn.owner_pid).or_insert_with(|| {
                (conn.process_name.clone(), conn.process_path.clone(), Vec::new())
            });

            entry.2.push(ProcessSocketConnection {
                local_addr: conn.local_addr.clone(),
                remote_addr: conn.remote_addr.clone(),
                state: conn.state.clone(),
                ip_version: conn.ip_version.clone(),
                is_new,
            });
        }

        self.previous_connections = current_conns_set;

        let mut reports = Vec::new();

        for (pid, (name, path, conns)) in grouped {
            let active_count = conns.len() as u32;

            // Query process I/O counters
            let (read_bytes, write_bytes) = query_process_io_counters(pid).unwrap_or((0, 0));

            let (read_rate_bps, write_rate_bps) = if let Some(&(prev_time, prev_read, prev_write)) = self.previous_io_samples.get(&pid) {
                let dt = now.duration_since(prev_time).as_secs_f64();
                if dt > 0.05 {
                    let d_read = read_bytes.saturating_sub(prev_read);
                    let d_write = write_bytes.saturating_sub(prev_write);
                    ((d_read as f64 * 8.0) / dt, (d_write as f64 * 8.0) / dt)
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };

            self.previous_io_samples.insert(pid, (now, read_bytes, write_bytes));

            // Streaming detection heuristic
            let (is_stream, confidence, reason) = if let Some(det) = detector {
                let remote_addrs: Vec<String> = conns.iter().map(|c| c.remote_addr.clone()).collect();
                det.classify_process(&name, active_count, &remote_addrs, read_rate_bps)
            } else {
                evaluate_streaming_heuristic(&name, &conns, read_rate_bps)
            };

            reports.push(ProcessNetworkReport {
                pid,
                name,
                path,
                active_connections: active_count,
                connections: conns,
                io_read_transfer_bytes: read_bytes,
                io_write_transfer_bytes: write_bytes,
                io_read_rate_bps: read_rate_bps,
                io_write_rate_bps: write_rate_bps,
                is_streaming: is_stream,
                streaming_confidence: confidence,
                streaming_reason: reason,
                bandwidth_measurement_type: "Combined Process I/O Transfer Rate (Includes file and socket transfers via Win32 GetProcessIoCounters)".to_string(),
            });
        }

        // Sort by active sockets descending
        reports.sort_by(|a, b| b.active_connections.cmp(&a.active_connections));
        reports
    }
}

fn evaluate_streaming_heuristic(
    name: &str,
    conns: &[ProcessSocketConnection],
    read_rate_bps: f64,
) -> (bool, String, Option<String>) {
    let name_lower = name.to_lowercase();

    let is_dedicated_stream_app = name_lower.contains("spotify")
        || name_lower.contains("netflix")
        || name_lower.contains("vlc")
        || name_lower.contains("prime");

    let is_browser = name_lower.contains("chrome")
        || name_lower.contains("edge")
        || name_lower.contains("firefox")
        || name_lower.contains("brave")
        || name_lower.contains("opera");

    // Check if connected to HTTPS port (443)
    let https_connections = conns
        .iter()
        .filter(|c| c.remote_addr.ends_with(":443") && c.state == "ESTABLISHED")
        .count();

    if is_dedicated_stream_app && !conns.is_empty() {
        (
            true,
            "high".to_string(),
            Some(format!("Dedicated streaming media process with {} active sockets.", conns.len())),
        )
    } else if is_browser && https_connections >= 3 && read_rate_bps > 1_500_000.0 {
        // High sustained I/O read (> 1.5 Mbps) over multiple HTTPS streams is typical of DASH/HLS video streaming
        (
            true,
            "high".to_string(),
            Some(format!(
                "Active video stream detected ({:.1} Mbps sustained read across {} HTTPS streams).",
                read_rate_bps / 1_000_000.0,
                https_connections
            )),
        )
    } else if is_browser && https_connections >= 2 {
        (
            true,
            "medium".to_string(),
            Some(format!("Web browser maintaining {} persistent secure media sockets.", https_connections)),
        )
    } else if is_browser && !conns.is_empty() {
        (
            false,
            "low".to_string(),
            Some("Web browser with background socket connections.".to_string()),
        )
    } else {
        (false, "none".to_string(), None)
    }
}
