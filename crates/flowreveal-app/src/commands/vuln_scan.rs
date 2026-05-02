use serde::{Deserialize, Serialize};
use tauri::{command, State};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityScanResult {
    pub total_scanned: usize,
    pub vulnerabilities: Vec<Vulnerability>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub session_id: u64,
    pub vuln_type: String,
    pub severity: VulnSeverity,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl std::fmt::Display for VulnSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VulnSeverity::Critical => write!(f, "Critical"),
            VulnSeverity::High => write!(f, "High"),
            VulnSeverity::Medium => write!(f, "Medium"),
            VulnSeverity::Low => write!(f, "Low"),
            VulnSeverity::Info => write!(f, "Info"),
        }
    }
}

#[command]
pub async fn scan_vulnerabilities(state: State<'_, AppState>) -> Result<VulnerabilityScanResult, String> {
    let sessions = state.sessions.read().await;
    let mut vulns = Vec::new();

    for session in sessions.iter() {
        let url = session.request.url.clone().unwrap_or_default();
        let url_lower = url.to_lowercase();

        if url_lower.contains("select%20") || url_lower.contains("union%20select") || url_lower.contains("or%201=1") || url_lower.contains("'--") {
            vulns.push(Vulnerability {
                session_id: session.id,
                vuln_type: "SQLInjection".to_string(),
                severity: VulnSeverity::Critical,
                title: "潜在 SQL 注入".to_string(),
                description: "URL 参数中包含 SQL 注入特征".to_string(),
                evidence: url.clone(),
                remediation: "使用参数化查询，不要拼接 SQL 语句".to_string(),
            });
        }

        if url_lower.contains("<script>") || url_lower.contains("javascript:") || url_lower.contains("onerror=") || url_lower.contains("onload=") {
            vulns.push(Vulnerability {
                session_id: session.id,
                vuln_type: "XSS".to_string(),
                severity: VulnSeverity::High,
                title: "潜在 XSS 攻击".to_string(),
                description: "URL 参数中包含跨站脚本特征".to_string(),
                evidence: url.clone(),
                remediation: "对用户输入进行 HTML 实体编码，使用 CSP 策略".to_string(),
            });
        }

        if let Some(body) = &session.request.body {
            let body_str = String::from_utf8_lossy(body).to_lowercase();
            if body_str.contains("password") || body_str.contains("secret") || body_str.contains("api_key") || body_str.contains("apikey") {
                if body_str.contains("\"password\"") || body_str.contains("\"secret\"") || body_str.contains("\"api_key\"") {
                    vulns.push(Vulnerability {
                        session_id: session.id,
                        vuln_type: "SensitiveDataExposure".to_string(),
                        severity: VulnSeverity::Medium,
                        title: "请求体包含敏感字段".to_string(),
                        description: "请求体中包含密码、密钥等敏感字段".to_string(),
                        evidence: "请求体包含敏感字段名称".to_string(),
                        remediation: "确保敏感数据通过安全通道传输，避免明文传输".to_string(),
                    });
                }
            }
        }

        if let Some(resp) = &session.response {
            for (key, value) in &resp.headers {
                let kl = key.to_lowercase();
                if kl == "access-control-allow-origin" && value == "*" {
                    if let Some(resp2) = &session.response {
                        let has_creds = resp2.headers.iter().any(|(k, v)| {
                            k.eq_ignore_ascii_case("access-control-allow-credentials") && v == "true"
                        });
                        if has_creds {
                            vulns.push(Vulnerability {
                                session_id: session.id,
                                vuln_type: "CORS".to_string(),
                                severity: VulnSeverity::High,
                                title: "CORS 配置危险".to_string(),
                                description: "Allow-Origin: * 且 Allow-Credentials: true，可导致凭据泄露".to_string(),
                                evidence: "Access-Control-Allow-Origin: * + Credentials: true".to_string(),
                                remediation: "不要同时设置 Allow-Origin: * 和 Allow-Credentials: true".to_string(),
                            });
                        } else {
                            vulns.push(Vulnerability {
                                session_id: session.id,
                                vuln_type: "CORS".to_string(),
                                severity: VulnSeverity::Low,
                                title: "CORS 配置宽松".to_string(),
                                description: "Access-Control-Allow-Origin: *，允许任何来源访问".to_string(),
                                evidence: "Access-Control-Allow-Origin: *".to_string(),
                                remediation: "限制 Allow-Origin 为可信域名".to_string(),
                            });
                        }
                    }
                }
            }

            let resp_headers_lower: Vec<String> = resp.headers.iter().map(|(k, _)| k.to_lowercase()).collect();
            let missing_headers: Vec<&str> = vec!["x-frame-options", "x-content-type-options", "strict-transport-security", "content-security-policy"]
                .into_iter()
                .filter(|h| !resp_headers_lower.iter().any(|rh| rh == *h))
                .collect();

            if !missing_headers.is_empty() && session.request.scheme == engine_core::http_message::Scheme::Https {
                vulns.push(Vulnerability {
                    session_id: session.id,
                    vuln_type: "MissingSecurityHeaders".to_string(),
                    severity: VulnSeverity::Low,
                    title: "缺少安全响应头".to_string(),
                    description: format!("缺少以下安全头: {}", missing_headers.join(", ")),
                    evidence: format!("缺失: {}", missing_headers.join(", ")),
                    remediation: "添加标准安全响应头以增强防护".to_string(),
                });
            }

            if let Some(body) = &resp.body {
                let body_str = String::from_utf8_lossy(body);
                if body_str.contains("stack trace") || body_str.contains("exception") || body_str.contains("error at") {
                    if resp.status_code.unwrap_or(0) >= 500 {
                        vulns.push(Vulnerability {
                            session_id: session.id,
                            vuln_type: "InfoLeakage".to_string(),
                            severity: VulnSeverity::Medium,
                            title: "错误页面信息泄露".to_string(),
                            description: "服务器错误响应中包含堆栈跟踪或内部信息".to_string(),
                            evidence: "响应体包含错误详情".to_string(),
                            remediation: "生产环境应返回通用错误页面，不暴露内部细节".to_string(),
                        });
                    }
                }
            }
        }

        if url_lower.starts_with("http://") {
            let has_sensitive = session.request.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("authorization"));
            if has_sensitive {
                vulns.push(Vulnerability {
                    session_id: session.id,
                    vuln_type: "InsecureTransport".to_string(),
                    severity: VulnSeverity::High,
                    title: "敏感数据通过 HTTP 传输".to_string(),
                    description: "包含认证信息的请求通过不安全的 HTTP 协议传输".to_string(),
                    evidence: url.clone(),
                    remediation: "所有包含敏感信息的请求应使用 HTTPS".to_string(),
                });
            }
        }
    }

    let total = sessions.len();
    let critical = vulns.iter().filter(|v| matches!(v.severity, VulnSeverity::Critical)).count();
    let high = vulns.iter().filter(|v| matches!(v.severity, VulnSeverity::High)).count();
    let medium = vulns.iter().filter(|v| matches!(v.severity, VulnSeverity::Medium)).count();
    let low = vulns.iter().filter(|v| matches!(v.severity, VulnSeverity::Low)).count();

    let summary = format!(
        "扫描了 {} 个请求，发现 {} 个漏洞：{} 严重、{} 高危、{} 中危、{} 低危",
        total, vulns.len(), critical, high, medium, low
    );

    Ok(VulnerabilityScanResult {
        total_scanned: total,
        vulnerabilities: vulns,
        summary,
    })
}
