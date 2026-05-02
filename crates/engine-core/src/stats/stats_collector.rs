use std::collections::HashMap;
use crate::http_message::HttpSession;
use super::stats_types::*;

pub struct StatsCollector;

impl StatsCollector {
    pub fn collect(sessions: &[HttpSession]) -> TrafficStats {
        let mut stats = TrafficStats::default();
        if sessions.is_empty() { return stats; }

        let mut domain_map: HashMap<String, (u64, u64, u64, u64)> = HashMap::new();
        let mut ct_map: HashMap<String, (u64, u64)> = HashMap::new();
        let mut proc_map: HashMap<String, (u64, u64)> = HashMap::new();
        let mut total_duration: u64 = 0;
        let mut duration_count: u64 = 0;
        let mut error_count: u64 = 0;

        for session in sessions {
            let req = &session.request;
            let resp = session.response.as_ref();

            stats.total_requests += 1;
            stats.total_bytes_sent += req.body_size as u64;
            if let Some(resp) = resp {
                stats.total_bytes_received += resp.body_size as u64;
            }

            if let Some(method) = &req.method {
                *stats.by_method.entry(method.clone()).or_insert(0) += 1;
            }

            if let Some(resp) = resp {
                if let Some(code) = resp.status_code {
                    *stats.by_status_code.entry(code).or_insert(0) += 1;
                    if code >= 400 { error_count += 1; }
                }
            }

            if let Some(host) = req.host() {
                let entry = domain_map.entry(host.to_string()).or_insert((0, 0, 0, 0));
                entry.0 += 1;
                entry.1 += req.body_size as u64 + resp.map(|r| r.body_size as u64).unwrap_or(0);
                if let Some(dur) = session.duration_us() {
                    entry.2 += dur;
                    entry.3 += 1;
                }
            }

            let ct = req.content_type.clone()
                .or_else(|| resp.and_then(|r| r.content_type.clone()))
                .unwrap_or_else(|| "unknown".to_string());
            let ct_key = ct.split(';').next().unwrap_or("unknown").trim().to_string();
            let ct_entry = ct_map.entry(ct_key).or_insert((0, 0));
            ct_entry.0 += 1;
            ct_entry.1 += req.body_size as u64 + resp.map(|r| r.body_size as u64).unwrap_or(0);

            if let Some(proc) = &req.process_name {
                let entry = proc_map.entry(proc.clone()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += req.body_size as u64 + resp.map(|r| r.body_size as u64).unwrap_or(0);
            }

            if let Some(dur) = session.duration_us() {
                total_duration += dur;
                duration_count += 1;
                let ms = dur as f64 / 1000.0;
                if ms < 100.0 { stats.duration_distribution[0].count += 1; }
                else if ms < 500.0 { stats.duration_distribution[1].count += 1; }
                else if ms < 1000.0 { stats.duration_distribution[2].count += 1; }
                else if ms < 5000.0 { stats.duration_distribution[3].count += 1; }
                else { stats.duration_distribution[4].count += 1; }
            }

            let total_size = req.body_size as u64 + resp.map(|r| r.body_size as u64).unwrap_or(0);
            if total_size < 1024 { stats.size_distribution[0].count += 1; }
            else if total_size < 10240 { stats.size_distribution[1].count += 1; }
            else if total_size < 102400 { stats.size_distribution[2].count += 1; }
            else if total_size < 1048576 { stats.size_distribution[3].count += 1; }
            else { stats.size_distribution[4].count += 1; }
        }

        stats.avg_duration_us = if duration_count > 0 { total_duration / duration_count } else { 0 };
        stats.error_rate = if stats.total_requests > 0 { (error_count as f64 / stats.total_requests as f64) * 100.0 } else { 0.0 };

        let total = stats.total_requests.max(1) as f64;
        for bucket in &mut stats.duration_distribution {
            bucket.percentage = (bucket.count as f64 / total) * 100.0;
        }
        for bucket in &mut stats.size_distribution {
            bucket.percentage = (bucket.count as f64 / total) * 100.0;
        }

        let mut domains: Vec<_> = domain_map.into_iter().map(|(domain, (count, bytes, dur_sum, dur_count))| {
            DomainStats {
                domain,
                request_count: count,
                total_bytes: bytes,
                avg_duration_us: if dur_count > 0 { dur_sum / dur_count } else { 0 },
                error_rate: 0.0,
            }
        }).collect();
        domains.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        stats.by_domain = domains.into_iter().take(20).collect();

        let mut content_types: Vec<_> = ct_map.into_iter().map(|(content_type, (count, bytes))| {
            ContentTypeStats {
                content_type,
                count,
                total_bytes: bytes,
                percentage: (count as f64 / total) * 100.0,
            }
        }).collect();
        content_types.sort_by(|a, b| b.count.cmp(&a.count));
        stats.by_content_type = content_types;

        let mut processes: Vec<_> = proc_map.into_iter().map(|(process_name, (count, bytes))| {
            ProcessStats { process_name, request_count: count, total_bytes: bytes }
        }).collect();
        processes.sort_by(|a, b| b.request_count.cmp(&a.request_count));
        stats.by_process = processes;

        stats
    }
}
