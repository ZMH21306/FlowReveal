use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::divert::nat_table::{NatEntryState, NatTable};
use crate::divert::packet_processor::{parse_packet, modify_outbound_dnat, modify_inbound_snat};

#[derive(Debug, Error)]
pub enum DivertError {
    #[error("WinDivert initialization failed: {0}")]
    InitFailed(String),
    #[error("WinDivert open failed: {0}")]
    OpenFailed(String),
    #[error("WinDivert receive failed: {0}")]
    RecvFailed(String),
    #[error("WinDivert send failed: {0}")]
    SendFailed(String),
    #[error("Administrator privileges required")]
    RequiresElevation,
    #[error("Wi-Fi adapter detected, may not work correctly")]
    WifiAdapter,
    #[error("Not running")]
    NotRunning,
    #[error("Already running")]
    AlreadyRunning,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivertConfig {
    pub proxy_port: u16,
    pub capture_ports: Vec<u16>,
    pub exclude_pids: Vec<u32>,
    pub include_pids: Vec<u32>,
    pub capture_localhost: bool,
}

impl Default for DivertConfig {
    fn default() -> Self {
        Self {
            proxy_port: 40961,
            capture_ports: vec![80, 443],
            exclude_pids: vec![],
            include_pids: vec![],
            capture_localhost: false,
        }
    }
}

pub struct PacketDiverter {
    config: DivertConfig,
    nat_table: Arc<NatTable>,
    _exclude_pids: HashSet<u32>,
    _include_pids: HashSet<u32>,
    running: Arc<AtomicBool>,
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl PacketDiverter {
    pub fn new(config: DivertConfig, nat_table: Arc<NatTable>) -> Result<Self, DivertError> {
        let self_pid = std::process::id();
        let mut exclude_pids: HashSet<u32> = config.exclude_pids.iter().copied().collect();
        exclude_pids.insert(self_pid);
        let include_pids: HashSet<u32> = config.include_pids.iter().copied().collect();

        Ok(Self {
            config,
            nat_table,
            _exclude_pids: exclude_pids,
            _include_pids: include_pids,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
        })
    }

    pub fn start(&mut self) -> Result<(), DivertError> {
        if self.running.load(Ordering::SeqCst) {
            return Err(DivertError::AlreadyRunning);
        }

        if !super::elevation::is_elevated() {
            return Err(DivertError::RequiresElevation);
        }

        tracing::info!(
            proxy_port = self.config.proxy_port,
            capture_ports = ?self.config.capture_ports,
            exclude_pids = ?self._exclude_pids,
            capture_localhost = self.config.capture_localhost,
            "[PacketDiverter] Starting"
        );

        self.running.store(true, Ordering::SeqCst);

        let filter = self.build_filter_string();
        tracing::info!(filter = %filter, "[PacketDiverter] WinDivert filter");

        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let nat_table = self.nat_table.clone();
        let exclude_pids = self._exclude_pids.clone();
        let include_pids = self._include_pids.clone();
        let proxy_port = self.config.proxy_port;
        let running = self.running.clone();

        std::thread::Builder::new()
            .name("windivert-packet-loop".to_string())
            .spawn(move || {
                packet_loop(filter, nat_table, exclude_pids, include_pids, proxy_port, running, shutdown_rx);
            })
            .map_err(|e| DivertError::InitFailed(format!("Failed to spawn packet loop thread: {}", e)))?;

        tracing::info!("[PacketDiverter] Started successfully");
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), DivertError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(DivertError::NotRunning);
        }

        tracing::info!("[PacketDiverter] Stopping");
        self.running.store(false, Ordering::SeqCst);

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        self.nat_table.clear();
        tracing::info!("[PacketDiverter] Stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn build_filter_string(&self) -> String {
        let port_conditions: Vec<String> = self
            .config
            .capture_ports
            .iter()
            .map(|p| format!("tcp.DstPort == {}", p))
            .collect();

        let outbound_filter = if port_conditions.is_empty() {
            "outbound and tcp".to_string()
        } else {
            format!("outbound and ({})", port_conditions.join(" or "))
        };

        let inbound_filter = format!("inbound and tcp.SrcPort == {}", self.config.proxy_port);

        if self.config.capture_localhost {
            let loopback_ports = port_conditions.join(" or ");
            format!("({} or {}) or (loopback and ({}))", outbound_filter, inbound_filter, loopback_ports)
        } else {
            format!("({} or {})", outbound_filter, inbound_filter)
        }
    }
}

fn packet_loop(
    filter: String,
    nat_table: Arc<NatTable>,
    exclude_pids: HashSet<u32>,
    include_pids: HashSet<u32>,
    proxy_port: u16,
    running: Arc<AtomicBool>,
    shutdown_rx: std::sync::mpsc::Receiver<()>,
) {
    use windivert::prelude::*;
    use windivert::layer::NetworkLayer;

    let divert = match WinDivert::network(&filter, 0, WinDivertFlags::default()) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!(error = ?e, "[PacketDiverter] WinDivert open failed");
            running.store(false, Ordering::SeqCst);
            return;
        }
    };

    tracing::info!("[PacketDiverter] WinDivert handle opened");

    let shutdown_handle = divert.shutdown_handle();

    let cleanup_interval = Duration::from_secs(30);
    let mut last_cleanup = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        if shutdown_rx.try_recv().is_ok() {
            tracing::info!("[PacketDiverter] Shutdown signal received");
            break;
        }

        let mut packet_buf = [0u8; 65535];

        let packet = match divert.recv(&mut packet_buf) {
            Ok(p) => p,
            Err(WinDivertError::Recv(WinDivertRecvError::NoData)) => {
                break;
            }
            Err(e) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                tracing::debug!(error = ?e, "[PacketDiverter] Recv error");
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
        };

        let is_outbound = packet.address.outbound();

        let mut packet_data = packet.data.into_owned();

        if is_outbound {
            if let Ok(parsed) = parse_packet(&mut packet_data) {
                let original_dst_ip = parsed.dst_ip;
                let original_dst_port = parsed.dst_port;
                let client_ip = parsed.src_ip;
                let client_port = parsed.src_port;

                nat_table.insert_outbound(
                    client_ip, client_port,
                    original_dst_ip, original_dst_port,
                    None,
                );

                if parsed.is_syn() {
                    nat_table.update_state(client_ip, client_port, NatEntryState::SynSent);
                } else if parsed.is_fin() || parsed.is_rst() {
                    nat_table.update_state(client_ip, client_port, NatEntryState::FinWait);
                } else if parsed.is_ack() {
                    if let Some(entry) = nat_table.get_entry(client_ip, client_port) {
                        if entry.state == NatEntryState::SynSent {
                            drop(entry);
                            nat_table.update_state(client_ip, client_port, NatEntryState::Established);
                        }
                    }
                }

                let localhost_ip = match client_ip {
                    IpAddr::V4(_) => IpAddr::from(std::net::Ipv4Addr::new(127, 0, 0, 1)),
                    IpAddr::V6(_) => IpAddr::from(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
                };

                match modify_outbound_dnat(&mut packet_data, localhost_ip, proxy_port) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::debug!(error = %e, "[PacketDiverter] DNAT modification failed, passing through");
                    }
                }
            }
        } else {
            if let Ok(parsed) = parse_packet(&mut packet_data) {
                if parsed.src_port == proxy_port {
                    let client_ip = parsed.dst_ip;
                    let client_port = parsed.dst_port;

                    if let Some(original_dest) = nat_table.get_original_dest(client_ip, client_port) {
                        match modify_inbound_snat(&mut packet_data, original_dest.ip, original_dest.port) {
                            Ok(_) => {
                                if parsed.is_fin() || parsed.is_rst() {
                                    nat_table.remove(client_ip, client_port);
                                }
                            }
                            Err(e) => {
                                tracing::debug!(error = %e, "[PacketDiverter] SNAT modification failed, passing through");
                            }
                        }
                    }
                }
            }
        }

        let mut send_packet = unsafe { windivert::packet::WinDivertPacket::<NetworkLayer>::new(packet_data) };
        send_packet.address.set_outbound(is_outbound);

        if let Err(e) = divert.send(&send_packet) {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            tracing::debug!(error = ?e, "[PacketDiverter] Send error");
        }

        if last_cleanup.elapsed() >= cleanup_interval {
            let removed = nat_table.cleanup_expired();
            if removed > 0 {
                tracing::debug!(removed, "[PacketDiverter] Cleaned up expired NAT entries");
            }
            last_cleanup = std::time::Instant::now();
        }
    }

    let _ = shutdown_handle.shutdown();
    drop(divert);
    tracing::info!("[PacketDiverter] Packet loop exited");
}

impl Drop for PacketDiverter {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            self.running.store(false, Ordering::SeqCst);
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
            tracing::info!("[PacketDiverter] Drop: auto-stopped");
        }
    }
}
