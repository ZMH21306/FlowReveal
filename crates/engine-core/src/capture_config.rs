use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub mode: CaptureMode,
    pub capture_http: bool,
    pub capture_https: bool,
    pub ports: Vec<u16>,
    pub process_filters: Vec<String>,
    pub host_filters: Vec<String>,
    pub max_body_size: usize,
    pub ca_cert_path: Option<String>,
    pub ca_key_path: Option<String>,
    pub mitm_bypass_hosts: Vec<String>,
    pub proxy_port: u16,
    pub transparent_proxy_port: u16,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::DualProxy,
            capture_http: true,
            capture_https: false,
            ports: vec![],
            process_filters: vec![],
            host_filters: vec![],
            max_body_size: 5 * 1024 * 1024,
            ca_cert_path: None,
            ca_key_path: None,
            mitm_bypass_hosts: vec![],
            proxy_port: 40960,
            transparent_proxy_port: 40961,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureMode {
    ForwardProxy,
    TransparentProxy,
    DualProxy,
    ApiHook,
}

impl std::fmt::Display for CaptureMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureMode::ForwardProxy => write!(f, "Forward Proxy"),
            CaptureMode::TransparentProxy => write!(f, "Transparent Proxy"),
            CaptureMode::DualProxy => write!(f, "Dual Proxy"),
            CaptureMode::ApiHook => write!(f, "API Hook"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureFilter {
    pub direction: FilterDirection,
    pub field: FilterField,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterDirection {
    Request,
    Response,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterField {
    Method,
    Url,
    Host,
    StatusCode,
    ContentType,
    HeaderName,
    HeaderValue,
    Body,
    ProcessName,
    Scheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    Matches,
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterGroup {
    pub logic: FilterLogic,
    pub filters: Vec<FilterGroupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterGroupItem {
    Filter(CaptureFilter),
    Group(FilterGroup),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterLogic {
    And,
    Or,
}

impl CaptureFilter {
    pub fn matches(&self, message: &crate::http_message::HttpMessage) -> bool {
        let field_value = match self.field {
            FilterField::Method => message.method.clone(),
            FilterField::Url => message.url.clone(),
            FilterField::Host => message.host().map(|h| h.to_string()),
            FilterField::StatusCode => message.status_code.map(|c| c.to_string()),
            FilterField::ContentType => message.content_type.clone(),
            FilterField::HeaderName | FilterField::HeaderValue => {
                let found = message.headers.iter().find(|(k, v)| {
                    if self.field == FilterField::HeaderName {
                        self.compare(k, &self.value)
                    } else {
                        self.compare(v, &self.value)
                    }
                });
                return found.is_some();
            }
            FilterField::Body => {
                if let Some(body) = &message.body {
                    String::from_utf8_lossy(body).to_string().into()
                } else {
                    None
                }
            }
            FilterField::ProcessName => message.process_name.clone(),
            FilterField::Scheme => Some(message.scheme.to_string()),
        };

        match field_value {
            Some(val) => self.compare(&val, &self.value),
            None => false,
        }
    }

    fn compare(&self, lhs: &str, rhs: &str) -> bool {
        match self.operator {
            FilterOperator::Equals => lhs.eq_ignore_ascii_case(rhs),
            FilterOperator::NotEquals => !lhs.eq_ignore_ascii_case(rhs),
            FilterOperator::Contains => lhs.to_lowercase().contains(&rhs.to_lowercase()),
            FilterOperator::NotContains => !lhs.to_lowercase().contains(&rhs.to_lowercase()),
            FilterOperator::StartsWith => lhs.to_lowercase().starts_with(&rhs.to_lowercase()),
            FilterOperator::EndsWith => lhs.to_lowercase().ends_with(&rhs.to_lowercase()),
            FilterOperator::Matches => {
                regex::Regex::new(rhs)
                    .map(|re| re.is_match(lhs))
                    .unwrap_or(false)
            }
            FilterOperator::GreaterThan | FilterOperator::LessThan => {
                let lhs_num: f64 = lhs.parse().unwrap_or(f64::NAN);
                let rhs_num: f64 = rhs.parse().unwrap_or(f64::NAN);
                if self.operator == FilterOperator::GreaterThan {
                    lhs_num > rhs_num
                } else {
                    lhs_num < rhs_num
                }
            }
        }
    }
}

impl FilterGroup {
    pub fn matches(&self, message: &crate::http_message::HttpMessage) -> bool {
        match self.logic {
            FilterLogic::And => self.filters.iter().all(|item| item.matches(message)),
            FilterLogic::Or => self.filters.iter().any(|item| item.matches(message)),
        }
    }
}

impl FilterGroupItem {
    pub fn matches(&self, message: &crate::http_message::HttpMessage) -> bool {
        match self {
            FilterGroupItem::Filter(f) => f.matches(message),
            FilterGroupItem::Group(g) => g.matches(message),
        }
    }
}
