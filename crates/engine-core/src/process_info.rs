use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub command_line: Option<String>,
    pub icon_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub protocol: TransportProtocol,
    pub state: TcpState,
    pub owning_pid: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
    DeleteTcb,
}

impl TcpState {
    pub fn from_mib_state(state: u32) -> Self {
        match state {
            1 => TcpState::Closed,
            2 => TcpState::Listen,
            3 => TcpState::SynSent,
            4 => TcpState::SynReceived,
            5 => TcpState::Established,
            6 => TcpState::FinWait1,
            7 => TcpState::FinWait2,
            8 => TcpState::CloseWait,
            9 => TcpState::Closing,
            10 => TcpState::LastAck,
            11 => TcpState::TimeWait,
            12 => TcpState::DeleteTcb,
            _ => TcpState::Closed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResolverConfig {
    pub poll_interval_ms: u64,
    pub cache_ttl_ms: u64,
    pub max_cache_entries: usize,
}

impl Default for ProcessResolverConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 500,
            cache_ttl_ms: 5000,
            max_cache_entries: 4096,
        }
    }
}
