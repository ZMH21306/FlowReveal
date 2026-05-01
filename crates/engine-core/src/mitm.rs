use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateAuthority {
    pub cert_pem: String,
    pub key_pem: String,
    pub serial_number: u64,
    pub not_before: u64,
    pub not_after: u64,
    pub is_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCert {
    pub cert_pem: String,
    pub key_pem: String,
    pub host: String,
    pub serial_number: u64,
    pub valid_from: u64,
    pub valid_to: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitmConfig {
    pub enabled: bool,
    pub bypass_hosts: Vec<String>,
    pub bypass_port_ranges: Vec<(u16, u16)>,
    pub max_cert_cache_entries: usize,
    pub cert_validity_days: u32,
}

impl Default for MitmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bypass_hosts: vec![
                "pinning.test".to_string(),
            ],
            bypass_port_ranges: vec![],
            max_cert_cache_entries: 1024,
            cert_validity_days: 365,
        }
    }
}

impl MitmConfig {
    pub fn should_bypass(&self, host: &str) -> bool {
        self.bypass_hosts.iter().any(|h| {
            host.eq_ignore_ascii_case(h) || host.ends_with(&format!(".{}", h))
        })
    }
}
