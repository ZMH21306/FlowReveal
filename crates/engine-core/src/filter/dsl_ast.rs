use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum DslExpr {
    FieldMatch { field: DslField, op: DslOp, value: DslValue },
    And(Box<DslExpr>, Box<DslExpr>),
    Or(Box<DslExpr>, Box<DslExpr>),
    Not(Box<DslExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DslField {
    Method,
    Url,
    Host,
    Path,
    Status,
    ProcessName,
    ProcessId,
    Body,
    ContentType,
    Scheme,
    Duration,
    Size,
    Header(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DslOp {
    Contains,
    Equals,
    NotEquals,
    Regex,
    GreaterThan,
    LessThan,
    Range,
    Wildcard,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DslValue {
    String(String),
    Number(f64),
    Range(f64, f64),
    SizeBytes(u64),
    DurationMs(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslParseError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for DslParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "位置 {}: {}", self.position, self.message)
    }
}

impl std::error::Error for DslParseError {}
