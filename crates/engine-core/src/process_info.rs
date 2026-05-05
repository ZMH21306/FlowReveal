use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub command_line: Option<String>,
    pub username: Option<String>,
    pub is_64_bit: Option<bool>,
}

impl ProcessInfo {
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            path: None,
            command_line: None,
            username: None,
            is_64_bit: None,
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

    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn with_is_64_bit(mut self, is_64_bit: bool) -> Self {
        self.is_64_bit = Some(is_64_bit);
        self
    }
}
