use std::collections::HashMap;
use std::fs;
use std::net::Ipv6Addr;
use crate::types::{SocketInfo, SocketProtocol};

pub fn parse_ipv4_addr(hex_str: &str) -> Option<String> {
    if hex_str.len() != 8 {
        return None;
    }
    let val = u32::from_str_radix(hex_str, 16).ok()?;
    let bytes = val.to_le_bytes();
    Some(format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]))
}

pub fn parse_ipv6_addr(hex_str: &str) -> Option<String> {
    if hex_str.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for i in 0..4 {
        let chunk = &hex_str[i * 8..(i + 1) * 8];
        let word = u32::from_str_radix(chunk, 16).ok()?;
        let word_bytes = word.to_le_bytes();
        bytes[i * 4..i * 4 + 4].copy_from_slice(&word_bytes);
    }
    let ip = Ipv6Addr::from(bytes);
    Some(ip.to_string())
}

pub fn tcp_state_name(st: u8) -> &'static str {
    match st {
        1 => "ESTABLISHED",
        2 => "SYN_SENT",
        3 => "SYN_RECV",
        4 => "FIN_WAIT1",
        5 => "FIN_WAIT2",
        6 => "TIME_WAIT",
        7 => "CLOSE",
        8 => "CLOSE_WAIT",
        9 => "LAST_ACK",
        10 => "LISTEN",
        11 => "CLOSING",
        12 => "NEW_SYN_RECV",
        _ => "UNKNOWN",
    }
}

pub fn udp_state_name(st: u8) -> &'static str {
    match st {
        1 => "ESTAB",
        7 => "UNCONN",
        _ => "UNCONN",
    }
}

pub fn get_socket_process_map() -> HashMap<u64, (u32, String)> {
    let mut map = HashMap::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return map;
    };

    for entry in entries.flatten() {
        let Ok(file_name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = file_name.parse::<u32>() else {
            continue;
        };

        let comm = fs::read_to_string(format!("/proc/{}/comm", pid))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let fd_dir = format!("/proc/{}/fd", pid);
        let Ok(fd_entries) = fs::read_dir(&fd_dir) else {
            continue;
        };

        for fd_entry in fd_entries.flatten() {
            if let Ok(target) = fs::read_link(fd_entry.path()) {
                let target_str = target.to_string_lossy();
                if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                    let inode_str = &target_str[8..target_str.len() - 1];
                    if let Ok(inode) = inode_str.parse::<u64>() {
                        map.insert(inode, (pid, comm.clone()));
                    }
                }
            }
        }
    }

    map
}

pub fn parse_socket_file(
    path: &str,
    protocol: SocketProtocol,
    proc_map: &HashMap<u64, (u32, String)>,
) -> Vec<SocketInfo> {
    let mut sockets = Vec::new();
    let Ok(content) = fs::read_to_string(path) else {
        return sockets;
    };

    let is_ipv6 = matches!(protocol, SocketProtocol::Tcp6 | SocketProtocol::Udp6);
    let is_udp = matches!(protocol, SocketProtocol::Udp | SocketProtocol::Udp6);

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        // Local address and port
        let mut local_parts = parts[1].split(':');
        let local_addr_hex = local_parts.next().unwrap_or("");
        let local_port_hex = local_parts.next().unwrap_or("");

        // Remote address and port
        let mut rem_parts = parts[2].split(':');
        let rem_addr_hex = rem_parts.next().unwrap_or("");
        let rem_port_hex = rem_parts.next().unwrap_or("");

        let local_addr = if is_ipv6 {
            parse_ipv6_addr(local_addr_hex).unwrap_or_else(|| local_addr_hex.to_string())
        } else {
            parse_ipv4_addr(local_addr_hex).unwrap_or_else(|| local_addr_hex.to_string())
        };

        let remote_addr = if is_ipv6 {
            parse_ipv6_addr(rem_addr_hex).unwrap_or_else(|| rem_addr_hex.to_string())
        } else {
            parse_ipv4_addr(rem_addr_hex).unwrap_or_else(|| rem_addr_hex.to_string())
        };

        let local_port = u16::from_str_radix(local_port_hex, 16).unwrap_or(0);
        let remote_port = u16::from_str_radix(rem_port_hex, 16).unwrap_or(0);

        let state_val = u8::from_str_radix(parts[3], 16).unwrap_or(0);
        let state = if is_udp {
            udp_state_name(state_val).to_string()
        } else {
            tcp_state_name(state_val).to_string()
        };

        let inode = parts[9].parse::<u64>().unwrap_or(0);
        let (pid, process_name) = if inode > 0 {
            if let Some((p, comm)) = proc_map.get(&inode) {
                (Some(*p), Some(comm.clone()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        sockets.push(SocketInfo {
            protocol: protocol.clone(),
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            inode,
            pid,
            process_name,
        });
    }

    sockets
}

fn socket_state_priority(state: &str) -> u8 {
    match state {
        "LISTEN" => 0,
        "ESTABLISHED" | "ESTAB" => 1,
        "SYN_SENT" | "SYN_RECV" => 2,
        "CLOSE_WAIT" | "FIN_WAIT1" | "FIN_WAIT2" | "TIME_WAIT" | "CLOSING" | "LAST_ACK" => 3,
        "UNCONN" | "CLOSE" => 4,
        _ => 5,
    }
}

pub fn get_active_sockets() -> Vec<SocketInfo> {
    let proc_map = get_socket_process_map();
    let mut sockets = Vec::new();

    sockets.extend(parse_socket_file("/proc/net/tcp", SocketProtocol::Tcp, &proc_map));
    sockets.extend(parse_socket_file("/proc/net/tcp6", SocketProtocol::Tcp6, &proc_map));
    sockets.extend(parse_socket_file("/proc/net/udp", SocketProtocol::Udp, &proc_map));
    sockets.extend(parse_socket_file("/proc/net/udp6", SocketProtocol::Udp6, &proc_map));

    sockets.sort_by(|a, b| {
        let p_a = socket_state_priority(&a.state);
        let p_b = socket_state_priority(&b.state);
        p_a.cmp(&p_b)
            .then_with(|| a.local_port.cmp(&b.local_port))
            .then_with(|| a.protocol.as_str().cmp(b.protocol.as_str()))
    });

    sockets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ipv4_addr() {
        assert_eq!(parse_ipv4_addr("0100007F"), Some("127.0.0.1".to_string()));
        assert_eq!(parse_ipv4_addr("00000000"), Some("0.0.0.0".to_string()));
        assert_eq!(parse_ipv4_addr("0101A8C0"), Some("192.168.1.1".to_string()));
        assert_eq!(parse_ipv4_addr("invalid"), None);
    }

    #[test]
    fn test_parse_ipv6_addr() {
        let loopback_hex = "00000000000000000000000001000000";
        assert_eq!(parse_ipv6_addr(loopback_hex), Some("::1".to_string()));
        let any_hex = "00000000000000000000000000000000";
        assert_eq!(parse_ipv6_addr(any_hex), Some("::".to_string()));
        assert_eq!(parse_ipv6_addr("short"), None);
    }

    #[test]
    fn test_tcp_and_udp_state_names() {
        assert_eq!(tcp_state_name(1), "ESTABLISHED");
        assert_eq!(tcp_state_name(10), "LISTEN");
        assert_eq!(tcp_state_name(6), "TIME_WAIT");
        assert_eq!(tcp_state_name(99), "UNKNOWN");

        assert_eq!(udp_state_name(7), "UNCONN");
        assert_eq!(udp_state_name(1), "ESTAB");
    }

    #[test]
    fn test_get_active_sockets() {
        let sockets = get_active_sockets();
        for s in &sockets {
            assert!(!s.local_addr.is_empty());
        }
    }
}
