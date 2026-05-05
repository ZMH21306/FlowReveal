use std::net::IpAddr;
use std::time::Instant;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_ENTRIES: usize = 65536;
const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnKey {
    pub client_ip: IpAddr,
    pub client_port: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct OriginalDest {
    pub ip: IpAddr,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatEntryState {
    SynSent,
    Established,
    FinWait,
}

#[derive(Clone)]
pub struct NatEntry {
    pub original_dest: OriginalDest,
    pub state: NatEntryState,
    pub last_activity: Instant,
    pub pid: Option<u32>,
}

pub struct NatTable {
    outbound: DashMap<ConnKey, NatEntry>,
    max_entries: usize,
    timeout_secs: u64,
}

impl NatTable {
    pub fn new(max_entries: usize) -> Self {
        Self {
            outbound: DashMap::with_capacity(max_entries.min(4096)),
            max_entries,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn insert_outbound(
        &self,
        client_ip: IpAddr,
        client_port: u16,
        original_dest_ip: IpAddr,
        original_dest_port: u16,
        pid: Option<u32>,
    ) {
        let key = ConnKey {
            client_ip,
            client_port,
        };
        let entry = NatEntry {
            original_dest: OriginalDest {
                ip: original_dest_ip,
                port: original_dest_port,
            },
            state: NatEntryState::SynSent,
            last_activity: Instant::now(),
            pid,
        };

        if self.outbound.len() >= self.max_entries {
            if let Some(oldest_key) = self
                .outbound
                .iter()
                .min_by_key(|e| e.value().last_activity)
                .map(|e| e.key().clone())
            {
                self.outbound.remove(&oldest_key);
            }
        }

        self.outbound.insert(key, entry);
    }

    pub fn get_original_dest(&self, client_ip: IpAddr, client_port: u16) -> Option<OriginalDest> {
        let key = ConnKey {
            client_ip,
            client_port,
        };
        self.outbound.get(&key).map(|e| e.original_dest)
    }

    pub fn get_entry(&self, client_ip: IpAddr, client_port: u16) -> Option<dashmap::mapref::one::Ref<'_, ConnKey, NatEntry>> {
        let key = ConnKey {
            client_ip,
            client_port,
        };
        self.outbound.get(&key)
    }

    pub fn update_state(&self, client_ip: IpAddr, client_port: u16, new_state: NatEntryState) {
        let key = ConnKey {
            client_ip,
            client_port,
        };
        if let Some(mut entry) = self.outbound.get_mut(&key) {
            entry.state = new_state;
            entry.last_activity = Instant::now();
        }
    }

    pub fn remove(&self, client_ip: IpAddr, client_port: u16) {
        let key = ConnKey {
            client_ip,
            client_port,
        };
        self.outbound.remove(&key);
    }

    pub fn cleanup_expired(&self) -> usize {
        let now = Instant::now();
        let timeout = std::time::Duration::from_secs(self.timeout_secs);
        let expired: Vec<ConnKey> = self
            .outbound
            .iter()
            .filter(|e| now.duration_since(e.value().last_activity) > timeout)
            .map(|e| e.key().clone())
            .collect();
        let count = expired.len();
        for key in expired {
            self.outbound.remove(&key);
        }
        count
    }

    pub fn len(&self) -> usize {
        self.outbound.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outbound.is_empty()
    }

    pub fn clear(&self) {
        self.outbound.clear();
    }
}

impl Default for NatTable {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_insert_and_lookup() {
        let table = NatTable::new(1024);
        table.insert_outbound(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)),
            12345,
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            443,
            Some(1234),
        );

        let dest = table
            .get_original_dest(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345)
            .unwrap();
        assert_eq!(dest.ip, IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)));
        assert_eq!(dest.port, 443);
    }

    #[test]
    fn test_lookup_miss() {
        let table = NatTable::new(1024);
        let result = table.get_original_dest(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 59999);
        assert!(result.is_none());
    }

    #[test]
    fn test_remove() {
        let table = NatTable::new(1024);
        table.insert_outbound(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)),
            12345,
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            443,
            None,
        );
        assert!(table.get_original_dest(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345).is_some());
        table.remove(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345);
        assert!(table.get_original_dest(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345).is_none());
    }

    #[test]
    fn test_update_state() {
        let table = NatTable::new(1024);
        table.insert_outbound(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)),
            12345,
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            443,
            None,
        );
        {
            let entry = table.get_entry(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345).unwrap();
            assert_eq!(entry.state, NatEntryState::SynSent);
        }
        table.update_state(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345, NatEntryState::Established);
        let entry = table.get_entry(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)), 12345).unwrap();
        assert_eq!(entry.state, NatEntryState::Established);
    }

    #[test]
    fn test_ipv6() {
        let table = NatTable::new(1024);
        let client_ip = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        let dest_ip = IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, 1));
        table.insert_outbound(client_ip, 54321, dest_ip, 443, Some(5678));

        let dest = table.get_original_dest(client_ip, 54321).unwrap();
        assert_eq!(dest.ip, dest_ip);
        assert_eq!(dest.port, 443);
    }

    #[test]
    fn test_max_entries_eviction() {
        let table = NatTable::new(3);
        for i in 1000..1004 {
            table.insert_outbound(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
                i,
                IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                443,
                None,
            );
        }
        assert_eq!(table.len(), 3);
    }

    #[test]
    fn test_clear() {
        let table = NatTable::new(1024);
        table.insert_outbound(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)),
            12345,
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            443,
            None,
        );
        assert!(!table.is_empty());
        table.clear();
        assert!(table.is_empty());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let table = Arc::new(NatTable::new(65536));
        let mut handles = vec![];

        for i in 0..4 {
            let table = Arc::clone(&table);
            handles.push(thread::spawn(move || {
                for j in 0..1000 {
                    let port = (i * 1000 + j) as u16;
                    table.insert_outbound(
                        IpAddr::V4(Ipv4Addr::new(192, 168, 1, i as u8)),
                        port,
                        IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                        443,
                        Some(i as u32),
                    );
                    let _ = table.get_original_dest(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i as u8)), port);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(table.len(), 4000);
    }
}
