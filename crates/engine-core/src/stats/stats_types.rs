use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficStats {
    pub total_requests: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub avg_duration_us: u64,
    pub error_rate: f64,
    pub by_domain: Vec<DomainStats>,
    pub by_content_type: Vec<ContentTypeStats>,
    pub by_status_code: HashMap<u16, u64>,
    pub by_method: HashMap<String, u64>,
    pub by_process: Vec<ProcessStats>,
    pub duration_distribution: Vec<DurationBucket>,
    pub size_distribution: Vec<SizeBucket>,
    pub timeline: Vec<TimelinePoint>,
}

impl Default for TrafficStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_bytes_sent: 0,
            total_bytes_received: 0,
            avg_duration_us: 0,
            error_rate: 0.0,
            by_domain: vec![],
            by_content_type: vec![],
            by_status_code: HashMap::new(),
            by_method: HashMap::new(),
            by_process: vec![],
            duration_distribution: vec![
                DurationBucket { range: "0-100ms".to_string(), count: 0, percentage: 0.0 },
                DurationBucket { range: "100-500ms".to_string(), count: 0, percentage: 0.0 },
                DurationBucket { range: "500ms-1s".to_string(), count: 0, percentage: 0.0 },
                DurationBucket { range: "1s-5s".to_string(), count: 0, percentage: 0.0 },
                DurationBucket { range: ">5s".to_string(), count: 0, percentage: 0.0 },
            ],
            size_distribution: vec![
                SizeBucket { range: "0-1KB".to_string(), count: 0, percentage: 0.0 },
                SizeBucket { range: "1-10KB".to_string(), count: 0, percentage: 0.0 },
                SizeBucket { range: "10-100KB".to_string(), count: 0, percentage: 0.0 },
                SizeBucket { range: "100KB-1MB".to_string(), count: 0, percentage: 0.0 },
                SizeBucket { range: ">1MB".to_string(), count: 0, percentage: 0.0 },
            ],
            timeline: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStats {
    pub domain: String,
    pub request_count: u64,
    pub total_bytes: u64,
    pub avg_duration_us: u64,
    pub error_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeStats {
    pub content_type: String,
    pub count: u64,
    pub total_bytes: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub process_name: String,
    pub request_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationBucket {
    pub range: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeBucket {
    pub range: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    pub timestamp: u64,
    pub request_count: u64,
    pub error_count: u64,
    pub avg_duration_us: u64,
}
