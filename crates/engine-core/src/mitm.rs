use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rcgen::{CertificateParams, DistinguishedName, KeyPair, IsCa, BasicConstraints, Ia5String};
use rcgen::SanType;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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

pub struct CaManager {
    ca_cert: rcgen::Certificate,
    ca_key_pair: KeyPair,
    ca_cert_pem: String,
    ca_key_pem: String,
    cert_cache: RwLock<HashMap<String, Arc<GeneratedCert>>>,
    max_cache: usize,
    validity_days: u32,
    serial_counter: AtomicU64,
    persist_dir: Option<PathBuf>,
}

impl CaManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, "FlowReveal CA");
        dn.push(rcgen::DnType::OrganizationName, "FlowReveal");
        dn.push(rcgen::DnType::CountryName, "CN");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
            rcgen::KeyUsagePurpose::DigitalSignature,
        ];

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        Ok(Self {
            ca_cert: cert,
            ca_key_pair: key_pair,
            ca_cert_pem: cert_pem.clone(),
            ca_key_pem: key_pem.clone(),
            cert_cache: RwLock::new(HashMap::new()),
            max_cache: 1024,
            validity_days: 365,
            serial_counter: AtomicU64::new(1),
            persist_dir: None,
        })
    }

    pub fn from_pem(ca_cert_pem: &str, ca_key_pem: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let key_pair = KeyPair::from_pem(ca_key_pem)?;
        let ca_cert_params = CertificateParams::from_ca_cert_pem(ca_cert_pem)?;
        let ca_cert = ca_cert_params.self_signed(&key_pair)?;

        Ok(Self {
            ca_cert,
            ca_key_pair: key_pair,
            ca_cert_pem: ca_cert_pem.to_string(),
            ca_key_pem: ca_key_pem.to_string(),
            cert_cache: RwLock::new(HashMap::new()),
            max_cache: 1024,
            validity_days: 365,
            serial_counter: AtomicU64::new(1),
            persist_dir: None,
        })
    }

    pub fn with_persist_dir(mut self, dir: PathBuf) -> Self {
        self.persist_dir = Some(dir);
        self
    }

    pub fn load_or_create(persist_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cert_path = persist_dir.join("flowreveal-ca.crt");
        let key_path = persist_dir.join("flowreveal-ca.key");

        if cert_path.exists() && key_path.exists() {
            let cert_pem = std::fs::read_to_string(&cert_path)?;
            let key_pem = std::fs::read_to_string(&key_path)?;

            match Self::from_pem(&cert_pem, &key_pem) {
                Ok(manager) => {
                    tracing::info!("Loaded existing CA certificate from {}", persist_dir.display());
                    return Ok(manager.with_persist_dir(persist_dir.to_path_buf()));
                }
                Err(e) => {
                    tracing::warn!("Failed to load existing CA cert (will regenerate): {}", e);
                }
            }
        }

        let manager = Self::new()?;
        manager.save_to_disk(persist_dir)?;
        tracing::info!("Generated new CA certificate and saved to {}", persist_dir.display());
        Ok(manager.with_persist_dir(persist_dir.to_path_buf()))
    }

    fn save_to_disk(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::fs::create_dir_all(dir)?;

        let cert_path = dir.join("flowreveal-ca.crt");
        let key_path = dir.join("flowreveal-ca.key");

        std::fs::write(&cert_path, &self.ca_cert_pem)?;
        std::fs::write(&key_path, &self.ca_key_pem)?;

        tracing::debug!("CA certificate saved to {} and {}", cert_path.display(), key_path.display());
        Ok(())
    }

    pub fn ca_certificate_authority(&self) -> CertificateAuthority {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        CertificateAuthority {
            cert_pem: self.ca_cert_pem.clone(),
            key_pem: self.ca_key_pem.clone(),
            serial_number: 1,
            not_before: now.saturating_sub(365 * 86400),
            not_after: now + 3650 * 86400,
            is_installed: false,
        }
    }

    pub fn ca_cert_der(&self) -> Vec<u8> {
        self.ca_cert.der().to_vec()
    }

    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    pub fn ca_key_pem(&self) -> &str {
        &self.ca_key_pem
    }

    pub async fn get_or_generate_cert(&self, host: &str) -> Result<Arc<GeneratedCert>, Box<dyn std::error::Error + Send + Sync>> {
        {
            let cache = self.cert_cache.read().await;
            if let Some(cert) = cache.get(host) {
                return Ok(Arc::clone(cert));
            }
        }

        let cert = self.generate_host_cert(host)?;

        {
            let mut cache = self.cert_cache.write().await;
            if cache.len() >= self.max_cache {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
            let cert = Arc::new(cert);
            cache.insert(host.to_string(), Arc::clone(&cert));
            Ok(cert)
        }
    }

    fn generate_host_cert(&self, host: &str) -> Result<GeneratedCert, Box<dyn std::error::Error + Send + Sync>> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, host);
        params.distinguished_name = dn;

        params.subject_alt_names = vec![
            SanType::DnsName(Ia5String::try_from(host.to_string())?),
        ];

        let wildcard_host: String = host.split('.').skip(1).collect::<Vec<_>>().join(".");
        if !wildcard_host.is_empty() {
            if let Ok(wildcard_san) = Ia5String::try_from(format!("*.{}", wildcard_host)) {
                params.subject_alt_names.push(SanType::DnsName(wildcard_san));
            }
        }

        params.is_ca = IsCa::NoCa;
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::DigitalSignature,
            rcgen::KeyUsagePurpose::KeyEncipherment,
        ];
        params.extended_key_usages = vec![
            rcgen::ExtendedKeyUsagePurpose::ServerAuth,
        ];

        let server_key = KeyPair::generate()?;
        let server_cert = params.signed_by(&server_key, &self.ca_cert, &self.ca_key_pair)?;

        let serial = self.serial_counter.fetch_add(1, Ordering::Relaxed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(GeneratedCert {
            cert_pem: server_cert.pem(),
            key_pem: server_key.serialize_pem(),
            host: host.to_string(),
            serial_number: serial,
            valid_from: now,
            valid_to: now + (self.validity_days as u64) * 86400,
        })
    }

    pub async fn clear_cache(&self) {
        self.cert_cache.write().await.clear();
    }

    pub async fn cache_size(&self) -> usize {
        self.cert_cache.read().await.len()
    }
}

pub fn build_tls_client_config() -> Result<rustls::ClientConfig, Box<dyn std::error::Error + Send + Sync>> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder_with_provider(
        std::sync::Arc::new(rustls::crypto::ring::default_provider()),
    )
        .with_safe_default_protocol_versions()?
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

pub fn build_tls_server_config(
    cert_der: Vec<u8>,
    key_der: Vec<u8>,
) -> Result<rustls::ServerConfig, Box<dyn std::error::Error + Send + Sync>> {
    let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
    let key = rustls::pki_types::PrivateKeyDer::from(
        rustls::pki_types::PrivatePkcs8KeyDer::from(key_der),
    );

    let config = rustls::ServerConfig::builder_with_provider(
        std::sync::Arc::new(rustls::crypto::ring::default_provider()),
    )
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)?;

    Ok(config)
}

pub fn pem_to_der(pem: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let pems = x509_parser::pem::Pem::iter_from_buffer(pem.as_bytes());
    for pem_item in pems {
        let item = pem_item?;
        if item.label == "CERTIFICATE" {
            return Ok(item.contents.to_vec());
        }
    }
    Err("No certificate found in PEM".into())
}

pub fn private_key_pem_to_der(pem: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let pems = x509_parser::pem::Pem::iter_from_buffer(pem.as_bytes());
    for pem_item in pems {
        let item = pem_item?;
        if item.label == "PRIVATE KEY" {
            return Ok(item.contents.to_vec());
        }
    }
    Err("No private key found in PEM".into())
}

pub fn extract_tls_info_from_server_name(
    server_name: &str,
    negotiated_protocol: Option<&str>,
) -> TlsNegotiatedInfo {
    TlsNegotiatedInfo {
        version: negotiated_protocol.unwrap_or("TLS 1.3").to_string(),
        cipher_suite: "AES_256_GCM_SHA384".to_string(),
        server_name: Some(server_name.to_string()),
    }
}

pub struct TlsNegotiatedInfo {
    pub version: String,
    pub cipher_suite: String,
    pub server_name: Option<String>,
}
