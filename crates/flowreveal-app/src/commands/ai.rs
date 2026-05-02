use serde::{Deserialize, Serialize};
use tauri::{command, State};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResult {
    pub summary: String,
    pub anomalies: Vec<Anomaly>,
    pub performance_insights: Vec<PerformanceInsight>,
    pub security_findings: Vec<SecurityFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub session_id: u64,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceInsight {
    pub session_id: Option<u64>,
    pub category: String,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    pub session_id: u64,
    pub finding_type: String,
    pub description: String,
    pub severity: String,
    pub evidence: String,
}

#[command]
pub async fn ai_analyze_traffic(state: State<'_, AppState>) -> Result<AiAnalysisResult, String> {
    let sessions = state.sessions.read().await;
    let mut anomalies = Vec::new();
    let mut performance_insights = Vec::new();
    let mut security_findings = Vec::new();

    let total = sessions.len();
    let mut error_count = 0u64;
    let mut slow_requests = Vec::new();
    let mut large_responses = Vec::new();

    for session in sessions.iter() {
        if let Some(resp) = &session.response {
            if let Some(code) = resp.status_code {
                if code >= 400 {
                    error_count += 1;
                }
                if code >= 500 {
                    anomalies.push(Anomaly {
                        session_id: session.id,
                        anomaly_type: "ServerError".to_string(),
                        description: format!("服务器错误: {} {}", code, resp.status_text.as_deref().unwrap_or("")),
                        severity: "high".to_string(),
                    });
                }
                if code == 401 || code == 403 {
                    security_findings.push(SecurityFinding {
                        session_id: session.id,
                        finding_type: "AuthFailure".to_string(),
                        description: format!("认证/授权失败: {}", code),
                        severity: "medium".to_string(),
                        evidence: session.request.url.clone().unwrap_or_default(),
                    });
                }
            }
            if let Some(duration) = resp.duration_us {
                if duration > 3_000_000 {
                    slow_requests.push((session.id, duration));
                    anomalies.push(Anomaly {
                        session_id: session.id,
                        anomaly_type: "SlowRequest".to_string(),
                        description: format!("请求耗时 {:.1}s", duration as f64 / 1_000_000.0),
                        severity: "medium".to_string(),
                    });
                }
            }
            if resp.body_size > 5_000_000 {
                large_responses.push((session.id, resp.body_size));
            }
        }

        for (key, value) in &session.request.headers {
            let kl = key.to_lowercase();
            if kl == "authorization" && value.starts_with("Bearer ") {
                let token = &value[7..];
                if token.len() > 20 {
                    security_findings.push(SecurityFinding {
                        session_id: session.id,
                        finding_type: "SensitiveToken".to_string(),
                        description: "Bearer Token 在请求头中明文传输".to_string(),
                        severity: "info".to_string(),
                        evidence: format!("Authorization: Bearer {}...", &token[..20.min(token.len())]),
                    });
                }
            }
            if kl == "cookie" && (value.contains("password") || value.contains("secret") || value.contains("token")) {
                security_findings.push(SecurityFinding {
                    session_id: session.id,
                    finding_type: "SensitiveCookie".to_string(),
                    description: "Cookie 中包含敏感信息".to_string(),
                    severity: "medium".to_string(),
                    evidence: format!("Cookie 包含: {}", &value[..100.min(value.len())]),
                });
            }
        }

        if let Some(resp) = &session.response {
            for (key, value) in &resp.headers {
                let kl = key.to_lowercase();
                if kl == "access-control-allow-origin" && value == "*" {
                    security_findings.push(SecurityFinding {
                        session_id: session.id,
                        finding_type: "PermissiveCORS".to_string(),
                        description: "CORS 配置过于宽松: Access-Control-Allow-Origin: *".to_string(),
                        severity: "low".to_string(),
                        evidence: "Access-Control-Allow-Origin: *".to_string(),
                    });
                }
                if kl == "x-frame-options" || kl == "content-security-policy" || kl == "strict-transport-security" {
                } else if kl.starts_with("x-") || kl == "content-security-policy" {
                }
            }
            let resp_headers_lower: Vec<String> = resp.headers.iter().map(|(k, _)| k.to_lowercase()).collect();
            if !resp_headers_lower.iter().any(|h| h == "x-frame-options") {
                security_findings.push(SecurityFinding {
                    session_id: session.id,
                    finding_type: "MissingSecurityHeader".to_string(),
                    description: "缺少 X-Frame-Options 安全头".to_string(),
                    severity: "low".to_string(),
                    evidence: "响应头中未找到 X-Frame-Options".to_string(),
                });
            }
            if !resp_headers_lower.iter().any(|h| h == "strict-transport-security") && session.request.scheme == engine_core::http_message::Scheme::Https {
                security_findings.push(SecurityFinding {
                    session_id: session.id,
                    finding_type: "MissingHSTS".to_string(),
                    description: "HTTPS 响应缺少 Strict-Transport-Security 头".to_string(),
                    severity: "low".to_string(),
                    evidence: "响应头中未找到 Strict-Transport-Security".to_string(),
                });
            }
        }
    }

    let error_rate = if total > 0 { (error_count as f64 / total as f64) * 100.0 } else { 0.0 };
    if error_rate > 10.0 {
        performance_insights.push(PerformanceInsight {
            session_id: None,
            category: "ErrorRate".to_string(),
            description: format!("错误率 {:.1}% ({} 个错误 / {} 总请求)", error_rate, error_count, total),
            recommendation: "检查服务器端日志，确认是否存在系统性问题".to_string(),
        });
    }

    if !slow_requests.is_empty() {
        let avg_slow: f64 = slow_requests.iter().map(|(_, d)| *d as f64 / 1_000_000.0).sum::<f64>() / slow_requests.len() as f64;
        performance_insights.push(PerformanceInsight {
            session_id: None,
            category: "Latency".to_string(),
            description: format!("{} 个请求超过 3 秒，平均耗时 {:.1}s", slow_requests.len(), avg_slow),
            recommendation: "考虑优化后端查询、增加缓存或使用 CDN".to_string(),
        });
    }

    if !large_responses.is_empty() {
        performance_insights.push(PerformanceInsight {
            session_id: None,
            category: "PayloadSize".to_string(),
            description: format!("{} 个响应超过 5MB", large_responses.len()),
            recommendation: "考虑启用压缩、分页或流式传输".to_string(),
        });
    }

    let summary = format!(
        "分析了 {} 个请求：发现 {} 个异常、{} 个性能洞察、{} 个安全发现。错误率 {:.1}%。",
        total, anomalies.len(), performance_insights.len(), security_findings.len(), error_rate
    );

    Ok(AiAnalysisResult {
        summary,
        anomalies,
        performance_insights,
        security_findings,
    })
}

#[command]
pub async fn ai_natural_language_query(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<u64>, String> {
    let sessions = state.sessions.read().await;
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for session in sessions.iter() {
        let mut matches = false;

        if query_lower.contains("慢") || query_lower.contains("slow") {
            if let Some(resp) = &session.response {
                if let Some(dur) = resp.duration_us {
                    if dur > 1_000_000 { matches = true; }
                }
            }
        }
        if query_lower.contains("错误") || query_lower.contains("error") || query_lower.contains("fail") {
            if let Some(resp) = &session.response {
                if let Some(code) = resp.status_code {
                    if code >= 400 { matches = true; }
                }
            }
        }
        if query_lower.contains("大") || query_lower.contains("large") || query_lower.contains("big") {
            if let Some(resp) = &session.response {
                if resp.body_size > 1_000_000 { matches = true; }
            }
        }
        if query_lower.contains("图片") || query_lower.contains("image") {
            if let Some(ct) = &session.response.as_ref().and_then(|r| r.content_type.as_ref()) {
                if ct.starts_with("image/") { matches = true; }
            }
        }
        if query_lower.contains("json") || query_lower.contains("api") {
            if let Some(ct) = &session.response.as_ref().and_then(|r| r.content_type.as_ref()) {
                if ct.contains("json") { matches = true; }
            }
        }
        if query_lower.contains("post") {
            if session.request.method.as_deref() == Some("POST") { matches = true; }
        }
        if query_lower.contains("安全") || query_lower.contains("security") || query_lower.contains("auth") {
            let has_auth = session.request.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("authorization"));
            let has_401_403 = session.response.as_ref().and_then(|r| r.status_code).map(|c| c == 401 || c == 403).unwrap_or(false);
            if has_auth || has_401_403 { matches = true; }
        }

        if matches {
            results.push(session.id);
        }
    }

    Ok(results)
}
