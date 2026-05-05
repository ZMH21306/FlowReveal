use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub command_line: Option<String>,
}

impl ProcessInfo {
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            path: None,
            command_line: None,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_command_line(mut self, command_line: String) -> Self {
        self.command_line = Some(command_line);
        self
    }
}
