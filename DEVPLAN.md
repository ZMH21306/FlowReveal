# FlowReveal 后续开发详细规划

> **版本**: v2.0  
> **基准日期**: 2026-05-02  
> **目标**: 全面超越 HTTP Debugger Pro，成为下一代 HTTP 调试器标杆  
> **当前进度**: 约 50-55%（阶段1-3核心完成，阶段4-6待开发）

---

## 目录

1. [当前状态总览](#1-当前状态总览)
2. [步骤一：核心规则系统](#步骤一核心规则系统)
3. [步骤二：高级过滤与搜索](#步骤二高级过滤与搜索)
4. [步骤三：性能统计与可视化](#步骤三性能统计与可视化)
5. [步骤四：编辑与辅助工具增强](#步骤四编辑与辅助工具增强)
6. [步骤五：协议支持增强](#步骤五协议支持增强)
7. [步骤六：性能优化与用户体验](#步骤六性能优化与用户体验)
8. [步骤七：创新功能与差异化竞争](#步骤七创新功能与差异化竞争)
9. [风险与依赖关系](#9-风险与依赖关系)
10. [里程碑与交付时间线](#10-里程碑与交付时间线)

---

## 1. 当前状态总览

### 1.1 已完成功能清单

| 模块 | 文件 | 状态 | 说明 |
|------|------|------|------|
| HTTP消息结构 | `engine-core/src/http_message.rs` | ✅ 完成 | `HttpMessage`/`HttpSession`/`WebSocketFrame`/`WsOpcode` |
| 拦截引擎Trait | `engine-core/src/intercept_engine.rs` | ✅ 完成 | `InterceptEngine` trait 定义 |
| 捕获配置 | `engine-core/src/capture_config.rs` | ✅ 完成 | `CaptureConfig`/`CaptureFilter`/`FilterGroup` 已含过滤结构 |
| 引擎统计 | `engine-core/src/engine_stats.rs` | ✅ 完成 | `EngineStats`/`EngineCapabilities`/`CaptureStatus` |
| CA证书管理 | `engine-core/src/mitm.rs` | ✅ 完成 | `CaManager` 生成/缓存/持久化/签发 |
| 正向代理 | `engine-core/src/proxy/forward_proxy.rs` | ✅ 完成 | HTTP代理 + CONNECT隧道 |
| MITM代理 | `engine-core/src/proxy/mitm_proxy.rs` | ✅ 完成 | HTTPS拦截/TLS握手/明文转发 |
| 透明代理 | `engine-core/src/proxy/transparent_proxy.rs` | ✅ 完成 | WFP重定向 |
| 平台抽象 | `engine-core/src/platform.rs` | ✅ 完成 | `PlatformCapture`/`PlatformHook` trait |
| 平台集成 | `engine-core/src/platform_integration/windows.rs` | ✅ 完成 | 系统代理/证书安装/WFP |
| HAR导出 | `engine-core/src/har_export.rs` | ✅ 完成 | HAR 1.2格式 |
| 请求重放 | `engine-core/src/replay.rs` | ✅ 完成 | HTTP/HTTPS重放 |
| Hook DLL | `hook-dll/src/lib.rs` + `winhttp.rs` | ⚠️ 初步 | 仅骨架 |
| IPC命令 | `flowreveal-app/src/commands/` | ✅ 完成 | capture/traffic/cert |
| 前端布局 | `src/components/layout/` | ✅ 完成 | AppShell/Toolbar/StatusBar |
| 请求列表 | `src/components/traffic/TrafficList.tsx` | ✅ 完成 | 方法/URL/状态/大小/耗时/进程 |
| 过滤栏 | `src/components/traffic/FilterBar.tsx` | ⚠️ 基础 | 仅文本+方法+协议+状态码 |
| 请求详情 | `src/components/detail/RequestDetail.tsx` | ✅ 完成 | Headers/Body/Cookie/Timing |
| 状态管理 | `src/store/index.ts` | ✅ 完成 | Zustand store + 基础过滤逻辑 |

### 1.2 关键缺失功能（对比 HTTP Debugger Pro）

| 优先级 | 功能 | HTTP Debugger Pro | FlowReveal | 影响 |
|--------|------|-------------------|------------|------|
| 🔴 P0 | 自动回复（Mock） | ✅ 完整 | ❌ 无 | 无法模拟后端响应 |
| 🔴 P0 | 修改消息头 | ✅ 完整 | ❌ 无 | 无法添加CORS/禁用缓存 |
| 🔴 P0 | 重定向规则 | ✅ 完整 | ❌ 无 | 无法重定向请求 |
| 🔴 P0 | 高级过滤DSL | ✅ 完整 | ⚠️ 基础 | 无法精确筛选流量 |
| 🟡 P1 | 性能统计面板 | ✅ 完整 | ❌ 无 | 无法分析流量模式 |
| 🟡 P1 | 数据转换器 | ✅ 完整 | ❌ 无 | 编码/解码不便 |
| 🟡 P1 | 书签/标记 | ✅ 完整 | ❌ 无 | 无法标记重要请求 |
| 🟡 P1 | 右键上下文菜单 | ✅ 完整 | ⚠️ 无 | 操作效率低 |
| 🟡 P1 | 全文搜索 | ⚠️ 部分 | ❌ 无 | 无法搜索Body内容 |
| 🟡 P1 | 高亮规则 | ✅ 完整 | ❌ 无 | 无法视觉区分请求 |
| 🟡 P1 | HTTP/2完整支持 | ⚠️ 部分 | ❌ 无 | 现代网站支持不足 |
| 🟡 P1 | WebSocket支持 | ❌ 无 | ❌ 无 | 双方均缺，我方需补齐 |
| 🟢 P2 | 多格式导出 | ✅ 完整 | ⚠️ 仅HAR | 导出灵活性不足 |
| 🟢 P2 | 虚拟滚动 | N/A | ❌ 无 | 大数据量卡顿 |

---

## 步骤一：核心规则系统

> **优先级**: 🔴 P0 — 最高优先级  
> **预计工期**: 4 周  
> **目标**: 实现自动回复、消息头修改、重定向三大规则功能，对标并超越 HTTP Debugger Pro 的规则系统

### 1.1 子步骤：规则数据结构设计（3天）

#### 任务描述

在 `engine-core` 中新建 `rules` 模块，定义所有规则相关的数据结构。这些结构需要同时满足后端匹配引擎和前端UI编辑的需求。

#### 新增文件

```
crates/engine-core/src/rules/
├── mod.rs
└── rule_types.rs
```

#### 详细设计

**`rule_types.rs` 核心类型**：

```rust
// 规则统一ID类型
pub type RuleId = u64;

// 规则大类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleCategory {
    AutoReply,
    HeaderModifier,
    Redirect,
}

// 规则统一包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub category: RuleCategory,
    pub enabled: bool,
    pub priority: u32,
    pub match_condition: MatchCondition,
    pub action: RuleAction,
    pub created_at: u64,
    pub updated_at: u64,
}

// 匹配条件（复用并扩展 capture_config 中已有的 CaptureFilter）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCondition {
    pub logic: FilterLogic,           // And / Or
    pub filters: Vec<MatchFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchFilter {
    pub field: MatchField,
    pub operator: MatchOperator,
    pub value: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchField {
    Method,
    Url,
    Host,
    Path,
    StatusCode,
    ContentType,
    HeaderName,
    HeaderValue,
    Body,
    ProcessName,
    Scheme,
    QueryParam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    MatchesRegex,
    GreaterThan,
    LessThan,
    InRange,       // 新增：值在范围内，value 格式 "min,max"
    Wildcard,      // 新增：通配符匹配，* 和 ?
}

// 规则动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    AutoReply(AutoReplyAction),
    HeaderModifier(HeaderModifierAction),
    Redirect(RedirectAction),
}

// 自动回复动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyAction {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body_source: BodySource,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodySource {
    Inline(String),
    File(String),
    Empty,
}

// 消息头修改动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderModifierAction {
    pub request_actions: Vec<HeaderAction>,
    pub response_actions: Vec<HeaderAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeaderAction {
    Add { name: String, value: String, only_if_missing: bool },
    Remove { name: String },
    Replace { name: String, value: String },
    ReplaceRegex { name: String, pattern: String, replacement: String },
}

// 重定向动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectAction {
    pub target_url: String,
    pub redirect_type: RedirectType,
    pub preserve_query: bool,
    pub preserve_path: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RedirectType {
    Permanent301,
    Temporary302,
    Temporary307,
    Permanent308,
}
```

**`mod.rs` 导出**：

```rust
pub mod rule_types;

pub use rule_types::*;
```

**修改 `lib.rs`**：添加 `pub mod rules;`

#### 验收标准

- [ ] `Rule` 结构体可序列化/反序列化为 JSON
- [ ] `RuleAction` 三种变体（AutoReply/HeaderModifier/Redirect）均可正确构造
- [ ] `MatchCondition` 支持嵌套逻辑（And/Or）
- [ ] `cargo build` 编译通过，无警告
- [ ] 编写单元测试：序列化 → 反序列化 → 断言相等

#### 测试用例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_roundtrip() {
        let rule = Rule {
            id: 1,
            name: "CORS Enable".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: FilterLogic::And,
                filters: vec![MatchFilter {
                    field: MatchField::Host,
                    operator: MatchOperator::Contains,
                    value: "api.example.com".to_string(),
                    case_sensitive: false,
                }],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![],
                response_actions: vec![
                    HeaderAction::Add {
                        name: "Access-Control-Allow-Origin".to_string(),
                        value: "*".to_string(),
                        only_if_missing: true,
                    },
                ],
            }),
            created_at: 0,
            updated_at: 0,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let de: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, rule.id);
        assert_eq!(de.name, rule.name);
    }
}
```

---

### 1.2 子步骤：规则匹配引擎（5天）

#### 任务描述

实现规则匹配核心逻辑：当代理收到请求/响应时，按优先级遍历所有启用的规则，匹配条件并执行动作。

#### 新增文件

```
crates/engine-core/src/rules/
├── rule_engine.rs       # 规则引擎总控
├── matcher.rs           # 条件匹配器
├── executor.rs          # 动作执行器
└── presets.rs           # 预设规则库
```

#### 详细设计

**`matcher.rs` — 条件匹配器**：

```rust
pub struct RuleMatcher;

impl RuleMatcher {
    /// 判断一条规则是否匹配给定的 HttpMessage
    pub fn matches(rule: &Rule, request: &HttpMessage, response: Option<&HttpMessage>) -> bool {
        if !rule.enabled {
            return false;
        }
        Self::match_condition(&rule.match_condition, request, response)
    }

    fn match_condition(
        cond: &MatchCondition,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> bool {
        match cond.logic {
            FilterLogic::And => cond.filters.iter().all(|f| Self::match_filter(f, request, response)),
            FilterLogic::Or => cond.filters.iter().any(|f| Self::match_filter(f, request, response)),
        }
    }

    fn match_filter(
        filter: &MatchFilter,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> bool {
        let target_value = match filter.field {
            MatchField::Method => request.method.clone(),
            MatchField::Url => request.url.clone(),
            MatchField::Host => request.host().map(|h| h.to_string()),
            MatchField::StatusCode => response.and_then(|r| r.status_code.map(|c| c.to_string())),
            MatchField::ContentType => request.content_type.clone().or_else(|| response.and_then(|r| r.content_type.clone())),
            MatchField::Body => request.body.as_ref().map(|b| String::from_utf8_lossy(b).to_string()),
            // ... 其他字段
            _ => None,
        };

        match target_value {
            Some(val) => Self::apply_operator(&val, &filter.value, &filter.operator, filter.case_sensitive),
            None => false,
        }
    }

    fn apply_operator(lhs: &str, rhs: &str, op: &MatchOperator, case_sensitive: bool) -> bool {
        let (l, r) = if case_sensitive {
            (lhs.to_string(), rhs.to_string())
        } else {
            (lhs.to_lowercase(), rhs.to_lowercase())
        };

        match op {
            MatchOperator::Equals => l == r,
            MatchOperator::NotEquals => l != r,
            MatchOperator::Contains => l.contains(&r),
            MatchOperator::NotContains => !l.contains(&r),
            MatchOperator::StartsWith => l.starts_with(&r),
            MatchOperator::EndsWith => l.ends_with(&r),
            MatchOperator::MatchesRegex => regex::Regex::new(rhs)
                .map(|re| re.is_match(lhs))
                .unwrap_or(false),
            MatchOperator::Wildcard => glob_match(&l, &r),
            MatchOperator::GreaterThan => l > r,
            MatchOperator::LessThan => l < r,
            MatchOperator::InRange => {
                let parts: Vec<&str> = rhs.split(',').collect();
                if parts.len() == 2 {
                    l >= parts[0].trim() && l <= parts[1].trim()
                } else {
                    false
                }
            }
        }
    }
}
```

**`executor.rs` — 动作执行器**：

```rust
pub enum RuleExecutionResult {
    AutoReply {
        status_code: u16,
        status_text: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        delay_ms: u64,
    },
    HeaderModified {
        modified_request_headers: Vec<(String, String)>,
        modified_response_headers: Option<Vec<(String, String)>>,
    },
    Redirected {
        new_url: String,
        redirect_type: RedirectType,
    },
    NoMatch,
}

pub struct RuleExecutor;

impl RuleExecutor {
    pub fn execute(action: &RuleAction) -> RuleExecutionResult {
        match action {
            RuleAction::AutoReply(a) => {
                let body = match &a.body_source {
                    BodySource::Inline(s) => s.as_bytes().to_vec(),
                    BodySource::File(path) => std::fs::read(path).unwrap_or_default(),
                    BodySource::Empty => vec![],
                };
                RuleExecutionResult::AutoReply {
                    status_code: a.status_code,
                    status_text: a.status_text.clone(),
                    headers: a.headers.clone(),
                    body,
                    delay_ms: a.delay_ms,
                }
            }
            RuleAction::HeaderModifier(a) => {
                RuleExecutionResult::HeaderModified {
                    modified_request_headers: vec![],  // 由调用方应用
                    modified_response_headers: None,
                }
            }
            RuleAction::Redirect(a) => {
                RuleExecutionResult::Redirected {
                    new_url: a.target_url.clone(),
                    redirect_type: a.redirect_type,
                }
            }
        }
    }
}
```

**`rule_engine.rs` — 规则引擎总控**：

```rust
pub struct RuleEngine {
    rules: RwLock<Vec<Rule>>,
    rule_counter: AtomicU64,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            rule_counter: AtomicU64::new(1),
        }
    }

    /// 添加规则
    pub async fn add_rule(&self, mut rule: Rule) -> RuleId {
        let id = self.rule_counter.fetch_add(1, Ordering::Relaxed);
        rule.id = id;
        self.rules.write().await.push(rule);
        id
    }

    /// 删除规则
    pub async fn remove_rule(&self, id: RuleId) -> bool {
        let mut rules = self.rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.id != id);
        rules.len() < before
    }

    /// 切换规则启用/禁用
    pub async fn toggle_rule(&self, id: RuleId, enabled: bool) -> bool {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// 获取所有规则（按优先级排序）
    pub async fn get_rules(&self) -> Vec<Rule> {
        let mut rules = self.rules.read().await.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    /// 对请求应用规则，返回第一个匹配的结果
    pub async fn apply(
        &self,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> Option<RuleExecutionResult> {
        let rules = self.rules.read().await;
        let mut sorted: Vec<&Rule> = rules.iter().filter(|r| r.enabled).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted {
            if RuleMatcher::matches(rule, request, response) {
                tracing::info!("[RuleEngine] 规则匹配: {} (id={})", rule.name, rule.id);
                return Some(RuleExecutor::execute(&rule.action));
            }
        }
        None
    }

    /// 导出规则为 JSON
    pub async fn export_rules(&self) -> String {
        let rules = self.get_rules().await;
        serde_json::to_string_pretty(&rules).unwrap_or_default()
    }

    /// 从 JSON 导入规则
    pub async fn import_rules(&self, json: &str) -> Result<usize, String> {
        let imported: Vec<Rule> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let count = imported.len();
        let mut rules = self.rules.write().await;
        for mut rule in imported {
            rule.id = self.rule_counter.fetch_add(1, Ordering::Relaxed);
            rules.push(rule);
        }
        Ok(count)
    }
}
```

**`presets.rs` — 预设规则库**：

```rust
impl RuleEngine {
    pub fn with_presets() -> Self {
        let engine = Self::new();
        // 预设规则将在启动时通过 async 方式添加
        engine
    }

    pub fn preset_cors_enable() -> Rule {
        Rule {
            id: 0,
            name: "添加 CORS 标头".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: false,
            priority: 10,
            match_condition: MatchCondition {
                logic: FilterLogic::And,
                filters: vec![],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![],
                response_actions: vec![
                    HeaderAction::Add { name: "Access-Control-Allow-Origin".into(), value: "*".into(), only_if_missing: true },
                    HeaderAction::Add { name: "Access-Control-Allow-Methods".into(), value: "GET, POST, PUT, DELETE, OPTIONS".into(), only_if_missing: true },
                    HeaderAction::Add { name: "Access-Control-Allow-Headers".into(), value: "*".into(), only_if_missing: true },
                    HeaderAction::Add { name: "Access-Control-Max-Age".into(), value: "86400".into(), only_if_missing: true },
                ],
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_cache_disable() -> Rule { /* ... */ }
    pub fn preset_cookies_remove() -> Rule { /* ... */ }
    pub fn preset_503_service_unavailable() -> Rule { /* ... */ }
    pub fn preset_302_redirect() -> Rule { /* ... */ }
    pub fn preset_200_ok() -> Rule { /* ... */ }
}
```

#### 验收标准

- [ ] `RuleMatcher::matches` 对所有 `MatchOperator` 变体正确匹配
- [ ] `RuleExecutor::execute` 对三种 `RuleAction` 正确生成 `RuleExecutionResult`
- [ ] `RuleEngine::apply` 按优先级返回第一个匹配结果
- [ ] 预设规则（CORS/禁用缓存/503/302/200）可正确生成
- [ ] 规则导入/导出 JSON 格式正确
- [ ] 单元测试覆盖率 > 80%
- [ ] `cargo build` 编译通过

#### 测试用例

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_auto_reply_rule() {
        let engine = RuleEngine::new();
        let rule = Rule {
            id: 0, name: "Mock 503".into(), category: RuleCategory::AutoReply,
            enabled: true, priority: 10,
            match_condition: MatchCondition {
                logic: FilterLogic::And,
                filters: vec![MatchFilter {
                    field: MatchField::Host, operator: MatchOperator::Contains,
                    value: "example.com".into(), case_sensitive: false,
                }],
            },
            action: RuleAction::AutoReply(AutoReplyAction {
                status_code: 503, status_text: "Service Unavailable".into(),
                headers: vec![], body_source: BodySource::Inline("Maintenance".into()),
                delay_ms: 0,
            }),
            created_at: 0, updated_at: 0,
        };
        engine.add_rule(rule).await;

        let request = HttpMessage { host: Some("api.example.com"), ..default_http_message() };
        let result = engine.apply(&request, None).await;
        assert!(matches!(result, Some(RuleExecutionResult::AutoReply { status_code: 503, .. })));
    }

    #[tokio::test]
    async fn test_priority_ordering() { /* 高优先级规则先匹配 */ }

    #[tokio::test]
    async fn test_disabled_rule_skipped() { /* 禁用规则不匹配 */ }

    #[tokio::test]
    async fn test_export_import_roundtrip() { /* 导出再导入后规则一致 */ }
}
```

---

### 1.3 子步骤：规则集成到代理流程（4天）

#### 任务描述

将规则引擎集成到 `forward_proxy.rs` 和 `mitm_proxy.rs` 的请求处理流程中，使规则在代理转发前生效。

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `engine-core/src/proxy/forward_proxy.rs` | 在 `handle_http_request` 和 `handle_connect_tunnel` 中插入规则检查点 |
| `engine-core/src/proxy/mitm_proxy.rs` | 在 `handle_mitm_connect` 的请求/响应处理中插入规则检查点 |
| `flowreveal-app/src/state.rs` | 添加 `RuleEngine` 到 `AppState` |
| `flowreveal-app/src/commands/capture.rs` | 将 `RuleEngine` 传递给代理启动函数 |

#### 详细设计

**代理流程中的规则检查点**：

```
请求到达代理
    │
    ▼
[检查点1] 自动回复规则
    │ 匹配 → 直接返回 Mock 响应，不转发到上游
    │ 不匹配 ↓
    ▼
[检查点2] 重定向规则
    │ 匹配 → 修改目标 URL，转发到新地址
    │ 不匹配 ↓
    ▼
[检查点3] 消息头修改规则（请求阶段）
    │ 匹配 → 修改请求头后转发
    │ 不匹配 ↓
    ▼
转发请求到上游服务器
    │
    ▼
收到上游响应
    │
    ▼
[检查点4] 消息头修改规则（响应阶段）
    │ 匹配 → 修改响应头后返回给客户端
    │ 不匹配 ↓
    ▼
返回响应给客户端
```

**`forward_proxy.rs` 修改示例**：

```rust
// 在 handle_http_request 中，构建 req_msg 之后、转发之前

// 检查自动回复规则
if let Some(rule_engine) = rule_engine.as_ref() {
    if let Some(RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms }) =
        rule_engine.apply(&req_msg, None).await
    {
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
        // 直接返回 Mock 响应给客户端
        write_response_to_client(&mut client_stream, status_code, &Some(status_text), &headers, &body).await?;
        // 发送 Mock 响应消息到 UI
        let mock_resp_msg = build_mock_response(session_id, status_code, &status_text, &headers, &body);
        let _ = engine_tx.send(mock_resp_msg).await;
        return Ok(());
    }
}
```

**`state.rs` 修改**：

```rust
pub struct AppState {
    // ... 已有字段 ...
    pub rule_engine: Arc<RuleEngine>,
}
```

#### 验收标准

- [ ] 设置自动回复规则后，匹配的请求直接返回 Mock 响应，不转发到上游
- [ ] 设置重定向规则后，请求被重定向到新地址
- [ ] 设置消息头修改规则后，请求/响应的 Headers 被正确修改
- [ ] 规则引擎不影响无规则时的正常代理功能
- [ ] 规则执行有日志记录（`tracing::info`）
- [ ] Mock 响应正确显示在 UI 中（包含状态码、Headers、Body）
- [ ] 端到端测试：启动代理 → 设置规则 → 发送请求 → 验证响应

---

### 1.4 子步骤：规则管理 Tauri IPC 命令（3天）

#### 任务描述

新增 Tauri IPC 命令，使前端可以管理规则（增删改查、启用/禁用、导入/导出）。

#### 新增文件

```
crates/flowreveal-app/src/commands/rules.rs
```

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `flowreveal-app/src/commands/mod.rs` | 添加 `pub mod rules;` |
| `flowreveal-app/src/lib.rs` | 在 `invoke_handler` 中注册规则命令 |

#### IPC 命令清单

```rust
// commands/rules.rs

#[command]
pub async fn add_rule(state: State<'_, AppState>, rule: Rule) -> Result<RuleId, String> {
    state.rule_engine.add_rule(rule).await.map_err(|e| e.to_string())
}

#[command]
pub async fn remove_rule(state: State<'_, AppState>, id: RuleId) -> Result<bool, String> {
    Ok(state.rule_engine.remove_rule(id).await)
}

#[command]
pub async fn toggle_rule(state: State<'_, AppState>, id: RuleId, enabled: bool) -> Result<bool, String> {
    Ok(state.rule_engine.toggle_rule(id, enabled).await)
}

#[command]
pub async fn get_rules(state: State<'_, AppState>) -> Result<Vec<Rule>, String> {
    Ok(state.rule_engine.get_rules().await)
}

#[command]
pub async fn update_rule(state: State<'_, AppState>, id: RuleId, rule: Rule) -> Result<bool, String> {
    state.rule_engine.update_rule(id, rule).await.map_err(|e| e.to_string())
}

#[command]
pub async fn enable_preset_rule(state: State<'_, AppState>, preset: PresetRuleType) -> Result<RuleId, String> {
    let rule = match preset {
        PresetRuleType::CorsEnable => RuleEngine::preset_cors_enable(),
        PresetRuleType::CacheDisable => RuleEngine::preset_cache_disable(),
        PresetRuleType::CookiesRemove => RuleEngine::preset_cookies_remove(),
        PresetRuleType::ServiceUnavailable503 => RuleEngine::preset_503_service_unavailable(),
        PresetRuleType::Redirect302 => RuleEngine::preset_302_redirect(),
        PresetRuleType::Ok200 => RuleEngine::preset_200_ok(),
    };
    let id = state.rule_engine.add_rule(rule).await;
    Ok(id)
}

#[command]
pub async fn export_rules(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.rule_engine.export_rules().await)
}

#[command]
pub async fn import_rules(state: State<'_, AppState>, json: String) -> Result<usize, String> {
    state.rule_engine.import_rules(&json).await.map_err(|e| e)
}
```

#### 前端绑定

```typescript
// src/lib/tauri-bindings.ts 新增

export async function addRule(rule: Rule): Promise<number> {
  return invoke<number>("add_rule", { rule });
}

export async function removeRule(id: number): Promise<boolean> {
  return invoke<boolean>("remove_rule", { id });
}

export async function toggleRule(id: number, enabled: boolean): Promise<boolean> {
  return invoke<boolean>("toggle_rule", { id, enabled });
}

export async function getRules(): Promise<Rule[]> {
  return invoke<Rule[]>("get_rules");
}

export async function updateRule(id: number, rule: Rule): Promise<boolean> {
  return invoke<boolean>("update_rule", { id, rule });
}

export async function enablePresetRule(preset: PresetRuleType): Promise<number> {
  return invoke<number>("enable_preset_rule", { preset });
}

export async function exportRules(): Promise<string> {
  return invoke<string>("export_rules");
}

export async function importRules(json: string): Promise<number> {
  return invoke<number>("import_rules", { json });
}
```

#### 验收标准

- [ ] 所有 IPC 命令可从前端成功调用
- [ ] `add_rule` → `get_rules` 返回包含新规则的列表
- [ ] `toggle_rule` 后规则状态正确变更
- [ ] `remove_rule` 后规则从列表消失
- [ ] `export_rules` → `import_rules` 往返一致
- [ ] 预设规则一键启用功能正常
- [ ] 错误情况（无效JSON、不存在的ID）返回友好错误信息

---

### 1.5 子步骤：规则管理 UI（5天）

#### 任务描述

实现规则管理的完整前端界面，包括规则列表、规则编辑对话框、预设规则快捷操作。

#### 新增文件

```
src/components/rules/
├── RulesPanel.tsx           # 规则管理主面板（侧边栏或Tab页）
├── RuleCard.tsx             # 单个规则卡片（名称/状态/开关/操作）
├── AutoReplyDialog.tsx      # 自动回复规则编辑对话框
├── HeaderModDialog.tsx      # 消息头修改规则编辑对话框
├── RedirectDialog.tsx       # 重定向规则编辑对话框
├── MatchConditionBuilder.tsx # 可视化条件构建器
├── PresetRulesList.tsx      # 预设规则快捷列表
└── RuleImportExport.tsx     # 规则导入/导出
src/hooks/
└── useRules.ts              # 规则管理 Hook
src/store/
└── rulesSlice.ts            # 规则状态管理（可选，或直接用 hooks）
```

#### UI 交互设计

**规则面板布局**：

```
┌─────────────────────────────────────────────────┐
│ 📋 规则管理                              [×] 关闭 │
├─────────────────────────────────────────────────┤
│ [自动回复] [修改标头] [重定向]     ← 标签页切换   │
├─────────────────────────────────────────────────┤
│ ⚡ 快捷预设                                      │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│ │ 🌐 CORS  │ │ 🚫 禁缓存│ │ 🍪 删Cookie│         │
│ └──────────┘ └──────────┘ └──────────┘          │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│ │ ⛔ 503   │ │ ↗️ 302   │ │ ✅ 200   │          │
│ └──────────┘ └──────────┘ └──────────┘          │
├─────────────────────────────────────────────────┤
│ + 新建自定义规则                                  │
├─────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────┐ │
│ │ 🟢 CORS Enable          [开关] [编辑] [删除] │ │
│ │   匹配: host contains api.example.com       │ │
│ │   动作: 添加 Access-Control-Allow-Origin: * │ │
│ ├─────────────────────────────────────────────┤ │
│ │ 🔴 Mock 503             [开关] [编辑] [删除] │ │
│ │   匹配: url matches /api/maintenance        │ │
│ │   动作: 返回 503 Service Unavailable         │ │
│ └─────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────┤
│ [导入规则] [导出规则]                              │
└─────────────────────────────────────────────────┘
```

**条件构建器**：

```
┌─────────────────────────────────────────────┐
│ 匹配条件                                     │
│ ┌──────────────────────────────────────────┐ │
│ │ [AND ▼]                                  │ │
│ │ ┌──────┐ ┌──────────┐ ┌───────────────┐ │ │
│ │ │ Host │ │ contains │ │ example.com   │ │ │
│ │ └──────┘ └──────────┘ └───────────────┘ │ │
│ │                              [×] 删除    │ │
│ │ ┌──────┐ ┌──────────┐ ┌───────────────┐ │ │
│ │ │ URL  │ │ regex    │ │ /api/v[0-9]+  │ │ │
│ │ └──────┘ └──────────┘ └───────────────┘ │ │
│ │                              [×] 删除    │ │
│ │ [+ 添加条件]                              │ │
│ └──────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

#### 验收标准

- [ ] 规则面板可通过工具栏按钮或快捷键打开
- [ ] 三个标签页（自动回复/修改标头/重定向）正确切换
- [ ] 预设规则一键启用后立即生效
- [ ] 自定义规则编辑器支持所有字段和操作符
- [ ] 条件构建器支持添加/删除条件行
- [ ] 规则开关可即时启用/禁用
- [ ] 规则导入/导出功能正常
- [ ] 规则列表为空时显示引导提示
- [ ] UI 响应流畅，无卡顿

---

## 步骤二：高级过滤与搜索

> **优先级**: 🔴 P0  
> **预计工期**: 2 周  
> **目标**: 实现强大的过滤 DSL 和全文搜索，大幅提升流量筛选效率

### 2.1 子步骤：过滤 DSL 解析器（4天）

#### 任务描述

实现一个类 Fiddler/Chrome DevTools 的过滤表达式解析器，支持 `method:GET host:api.example.com status:200..299` 等语法。

#### 新增文件

```
crates/engine-core/src/filter/
├── mod.rs
├── dsl_parser.rs          # DSL 词法+语法分析
├── dsl_ast.rs             # DSL 抽象语法树
└── dsl_matcher.rs         # DSL 匹配执行
```

#### DSL 语法规范

```
表达式 := 项 (逻辑运算符 项)*
项 := 字段运算符 值 | '(' 表达式 ')' | NOT 项
逻辑运算符 := AND | OR | '&&' | '||'
字段运算符 := 字段名 ':' 值 | 字段名 '=' 值 | 字段名 '=~' 正则 | 字段名 '>' 值 | 字段名 '<' 值 | 字段名 '..' 值
字段名 := method | url | host | path | status | code | proc | pid | body | header.NAME | content-type | scheme | duration | size
值 := 字符串 | 数字 | 范围(如 200..299)
NOT := '!' | 'NOT'
```

**示例表达式**：

```
method:GET AND host:api.example.com
status:200..299 OR status:404
proc:chrome AND NOT url:*\.png$
body:token AND size:>1KB
header.authorization:Bearer
duration:>1000ms
(method:POST OR method:PUT) AND content-type:json
```

#### 详细设计

**`dsl_ast.rs`**：

```rust
#[derive(Debug, Clone)]
pub enum DslExpr {
    FieldMatch { field: DslField, op: DslOp, value: DslValue },
    And(Box<DslExpr>, Box<DslExpr>),
    Or(Box<DslExpr>, Box<DslExpr>),
    Not(Box<DslExpr>),
}

#[derive(Debug, Clone)]
pub enum DslField {
    Method, Url, Host, Path, Status, ProcessName, ProcessId,
    Body, ContentType, Scheme, Duration, Size,
    Header(String),
}

#[derive(Debug, Clone)]
pub enum DslOp {
    Contains, Equals, NotEquals, Regex, GreaterThan, LessThan, Range, Wildcard,
}

#[derive(Debug, Clone)]
pub enum DslValue {
    String(String),
    Number(f64),
    Range(f64, f64),
    SizeBytes(u64),
    DurationMs(u64),
}
```

**`dsl_parser.rs`** — 递归下降解析器：

```rust
pub struct DslParser;

impl DslParser {
    pub fn parse(input: &str) -> Result<DslExpr, String> {
        let tokens = tokenize(input)?;
        let (expr, remaining) = parse_or(&tokens, 0)?;
        if remaining < tokens.len() {
            Err(format!("Unexpected token at position {}: {:?}", remaining, tokens[remaining]))
        } else {
            Ok(expr)
        }
    }
}
```

**`dsl_matcher.rs`**：

```rust
pub fn match_dsl(expr: &DslExpr, session: &HttpSession) -> bool {
    match expr {
        DslExpr::FieldMatch { field, op, value } => match_field(session, field, op, value),
        DslExpr::And(l, r) => match_dsl(l, session) && match_dsl(r, session),
        DslExpr::Or(l, r) => match_dsl(l, session) || match_dsl(r, session),
        DslExpr::Not(e) => !match_dsl(e, session),
    }
}
```

#### 验收标准

- [ ] 解析 `method:GET` → `FieldMatch { Method, Contains, String("GET") }`
- [ ] 解析 `status:200..299` → `FieldMatch { Status, Range, Range(200, 299) }`
- [ ] 解析 `method:POST AND host:api.example.com` → `And(...)`
- [ ] 解析 `(method:GET OR method:POST) AND NOT status:404` → 嵌套表达式
- [ ] 解析 `size:>1KB` → `FieldMatch { Size, GreaterThan, SizeBytes(1024) }`
- [ ] 解析 `duration:>1000ms` → `FieldMatch { Duration, GreaterThan, DurationMs(1000) }`
- [ ] 无效表达式返回清晰的错误信息
- [ ] `match_dsl` 对所有操作符正确匹配
- [ ] 单元测试覆盖率 > 90%

---

### 2.2 子步骤：前端过滤栏增强（3天）

#### 任务描述

将现有的简单 `FilterBar` 升级为支持 DSL 语法高亮、自动补全的高级过滤栏。

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `src/components/traffic/FilterBar.tsx` | 重写为高级过滤栏 |
| `src/store/index.ts` | 更新过滤逻辑，支持 DSL 表达式 |

#### 新增文件

```
src/components/traffic/
├── DslFilterBar.tsx         # DSL 过滤输入栏（语法高亮）
├── FilterSuggestions.tsx    # 输入建议/自动补全
└── SavedFilters.tsx         # 保存的过滤器
```

#### 功能特性

1. **语法高亮**：字段名（蓝色）、操作符（绿色）、值（白色）、逻辑运算符（橙色）
2. **自动补全**：输入时弹出字段名/操作符建议
3. **语法验证**：实时显示解析错误
4. **快捷过滤器**：预设按钮（仅XHR、仅错误、仅图片等）
5. **保存过滤器**：常用过滤表达式可保存命名

#### 验收标准

- [ ] 输入 DSL 表达式后请求列表实时过滤
- [ ] 语法错误时输入框下方显示红色提示
- [ ] 自动补全弹出字段名列表
- [ ] 快捷过滤器按钮一键过滤
- [ ] 保存的过滤器可从下拉列表选择
- [ ] 过滤性能：万级数据 < 100ms

---

### 2.3 子步骤：全文搜索引擎（3天）

#### 任务描述

实现后端全文搜索，支持在 URL、Headers、Body 中搜索关键词。

#### 新增文件

```
crates/engine-core/src/search/
├── mod.rs
├── search_index.rs         # 倒排索引
└── search_engine.rs        # 搜索引擎
```

#### 技术方案

使用简单的倒排索引（不引入 tantivy 等重量级库），支持：
- 关键词搜索（分词：按空格/标点切分）
- 正则搜索
- 大小写敏感/不敏感
- 搜索范围选择（URL / Headers / Body / 全部）

#### 新增 IPC 命令

```rust
#[command]
pub async fn search_traffic(
    state: State<'_, AppState>,
    query: String,
    scope: SearchScope,
    case_sensitive: bool,
    use_regex: bool,
) -> Result<Vec<u64>, String> { ... }

pub enum SearchScope {
    All,
    Url,
    Headers,
    Body,
}
```

#### 验收标准

- [ ] 搜索 "token" 返回所有包含该关键词的请求
- [ ] 搜索范围限定为 Body 时，不在 URL 中搜索
- [ ] 正则搜索 `Bearer\s+\w+` 正确匹配
- [ ] 大小写敏感选项正确生效
- [ ] 搜索结果高亮显示匹配位置
- [ ] 搜索性能：万级数据 < 200ms

---

## 步骤三：性能统计与可视化

> **优先级**: 🟡 P1  
> **预计工期**: 1.5 周  
> **目标**: 实现域名/类型/耗时/大小等多维统计分析面板

### 3.1 子步骤：统计引擎后端（3天）

#### 新增文件

```
crates/engine-core/src/stats/
├── mod.rs
├── stats_collector.rs      # 统计数据收集
└── stats_types.rs          # 统计数据类型
```

#### 统计数据结构

```rust
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
```

#### 新增 IPC 命令

```rust
#[command]
pub async fn get_traffic_stats(state: State<'_, AppState>) -> Result<TrafficStats, String> { ... }
```

#### 验收标准

- [ ] 统计数据随流量实时更新
- [ ] 域名统计 Top 20 正确排序
- [ ] 耗时分布直方图数据正确
- [ ] 时间线数据（QPS趋势）正确

---

### 3.2 子步骤：统计可视化 UI（4天）

#### 新增文件

```
src/components/stats/
├── StatsPanel.tsx           # 统计面板主界面
├── OverviewCards.tsx        # 总览卡片
├── DomainStatsChart.tsx     # 域名统计柱状图
├── ContentTypePie.tsx       # 内容类型饼图
├── DurationHistogram.tsx    # 耗时直方图
├── SizeDistribution.tsx     # 大小分布图
├── TimelineChart.tsx        # 时间线折线图
└── StatusCodeSummary.tsx    # 状态码汇总
```

#### 依赖安装

```bash
pnpm add recharts
```

#### 验收标准

- [ ] 总览卡片显示：总请求数、总数据量、平均耗时、错误率
- [ ] 域名统计柱状图 Top 20
- [ ] 内容类型饼图
- [ ] 耗时分布直方图（0-100ms / 100-500ms / 500ms-1s / 1s-5s / >5s）
- [ ] 时间线折线图（QPS趋势）
- [ ] 点击图表元素可筛选请求列表
- [ ] 图表交互：悬停显示详情、缩放

---

## 步骤四：编辑与辅助工具增强

> **优先级**: 🟡 P1  
> **预计工期**: 1.5 周  
> **目标**: 实现数据转换器、书签、右键菜单等编辑辅助功能

### 4.1 子步骤：数据转换器（2天）

#### 新增文件

```
src/components/tools/
└── DataTransformer.tsx
```

#### 支持的转换类型

| 类型 | 输入 → 输出 |
|------|------------|
| URL编码/解码 | `%E4%B8%AD` ↔ `中` |
| Base64编码/解码 | `5Lit` ↔ `中` |
| JSON格式化 | 压缩 ↔ 美化 |
| Unicode转义 | `\u4F60` ↔ `你` |
| 时间戳转换 | `1715000000` ↔ `2024-05-06...` |
| Hash计算 | 文本 → MD5/SHA1/SHA256 |

#### 验收标准

- [ ] 6种转换类型全部可用
- [ ] 双向转换（编码↔解码）
- [ ] 输入实时转换
- [ ] 一键复制结果

---

### 4.2 子步骤：书签系统（2天）

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `src/store/index.ts` | 添加 `bookmarks: Set<number>` 状态 |
| `src/components/traffic/TrafficList.tsx` | 添加书签图标列 |
| `src/components/detail/RequestDetail.tsx` | 添加书签按钮 |

#### 验收标准

- [ ] 点击星标图标添加/移除书签
- [ ] 书签请求在列表中特殊高亮
- [ ] 书签筛选按钮可只看书签请求
- [ ] 书签数据持久化到 localStorage

---

### 4.3 子步骤：右键上下文菜单（3天）

#### 新增文件

```
src/components/traffic/
└── ContextMenu.tsx
```

#### 菜单项

```
┌─ 复制 ──────────────────────┐
│  复制 URL                     │
│  复制 响应内容                │
│  复制 请求内容                │
│  复制 请求标头                │
│  复制为 cURL 命令            │
│  复制为 HAR 条目             │
├──────────────────────────────┤
│  过滤 ▸ 按此URL / 按此域名 / 排除 │
│  高亮 ▸ 高亮此类 / 高亮错误      │
│  自动回复 ▸ 503 / 200 / 自定义   │
│  修改标头 ▸ CORS / 禁缓存 / 删Cookie │
├──────────────────────────────┤
│  ⭐ 添加书签                  │
│  🌐 在浏览器打开              │
│  ↻ 重放请求                   │
│  📥 导出为HAR                 │
│  🗑 删除                      │
└──────────────────────────────┘
```

#### 验收标准

- [ ] 右键菜单在请求行上正确弹出
- [ ] 所有复制功能正确写入剪贴板
- [ ] "复制为 cURL" 生成可执行的 curl 命令
- [ ] 子菜单正确展开/收起
- [ ] 点击菜单外区域关闭菜单
- [ ] 快捷键 Ctrl+C 复制 URL

---

## 步骤五：协议支持增强

> **优先级**: 🟡 P1  
> **预计工期**: 3.5 周  
> **目标**: 完善 HTTP/2 和 WebSocket 支持

### 5.1 子步骤：HTTP/2 完整支持（2周）

#### 新增文件

```
crates/engine-core/src/protocol/
├── mod.rs
└── http2.rs
```

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `engine-core/src/proxy/mitm_proxy.rs` | 检测 ALPN h2 协商，走 HTTP/2 代理路径 |
| `engine-core/src/http_message.rs` | 添加 `stream_id` 字段到 `HttpMessage` |

#### 功能要点

- 检测客户端 ALPN 协商结果（h2 vs http/1.1）
- 使用 `h2` crate 处理多路复用流
- 每个 Stream ID 作为独立请求条目
- HPACK 头部压缩/解压记录
- SETTINGS 帧参数展示

#### 验收标准

- [ ] 访问 HTTP/2 网站时，多路复用流被分离为独立请求
- [ ] HTTP/2 请求在列表中显示协议为 "HTTP/2"
- [ ] 详情面板显示 Stream ID 和优先级信息
- [ ] HTTP/2 与 HTTP/1.1 混合场景正确处理
- [ ] 性能：HTTP/2 代理不引入明显延迟

---

### 5.2 子步骤：WebSocket 支持（1.5周）

#### 新增文件

```
crates/engine-core/src/protocol/
└── websocket.rs
```

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `engine-core/src/proxy/mitm_proxy.rs` | 检测 Upgrade: websocket，走 WS 拦截路径 |
| `src/components/detail/RequestDetail.tsx` | 添加 WebSocket 标签页 |

#### 新增文件（前端）

```
src/components/detail/
└── WebSocketView.tsx
```

#### 功能要点

- 识别 WebSocket Upgrade 握手
- 使用 `tokio-tungstenite` 建立双向 WS 代理
- 每条 Frame 记录到 `WebSocketFrame` 结构（已在 `http_message.rs` 中定义）
- 前端 WebSocket 视图显示消息流

#### 验收标准

- [ ] WebSocket 连接被正确识别和拦截
- [ ] 每条消息（Text/Binary/Ping/Pong/Close）被记录
- [ ] 消息方向（Outbound/Inbound）正确标识
- [ ] JSON 消息自动格式化显示
- [ ] 二进制消息以 Hex 视图显示
- [ ] WebSocket 连接关闭时状态正确更新

---

## 步骤六：性能优化与用户体验

> **优先级**: 🟡 P1  
> **预计工期**: 2 周  
> **目标**: 优化大数据量场景下的性能，提升用户体验

### 6.1 子步骤：虚拟滚动（3天）

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `src/components/traffic/TrafficList.tsx` | 引入 `@tanstack/react-virtual` 重写列表 |

#### 依赖安装

```bash
pnpm add @tanstack/react-virtual
```

#### 验收标准

- [ ] 10000 条数据流畅滚动（FPS > 55）
- [ ] 滚动位置在数据更新时保持稳定
- [ ] 选中行始终可见
- [ ] 内存占用不随数据量线性增长

---

### 6.2 子步骤：批量加载与分页（3天）

#### 修改文件

| 文件 | 修改内容 |
|------|---------|
| `flowreveal-app/src/commands/traffic.rs` | `get_requests` 支持 offset/limit 参数 |
| `src/store/index.ts` | 支持增量加载 |
| `src/hooks/useTraffic.ts` | 支持滚动到底部加载更多 |

#### 验收标准

- [ ] 初始加载只拉取最近 200 条
- [ ] 滚动到底部自动加载更多
- [ ] 全文搜索在后端执行
- [ ] 内存占用稳定在 200MB 以内

---

### 6.3 子步骤：主题系统（2天）

#### 新增文件

```
src/styles/theme.ts
src/hooks/useTheme.ts
```

#### 验收标准

- [ ] 亮色/暗色/系统跟随三种模式
- [ ] 切换主题无闪烁
- [ ] 主题偏好持久化到 localStorage
- [ ] 所有组件适配两种主题

---

### 6.4 子步骤：快捷键系统（2天）

#### 新增文件

```
src/lib/hotkeys.ts
```

#### 默认快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+F` | 聚焦搜索框 |
| `Ctrl+E` | 清除所有数据 |
| `Ctrl+S` | 保存会话 |
| `Space` | 开始/停止捕获 |
| `↑/↓` | 列表导航 |
| `Enter` | 查看详情 |
| `Esc` | 关闭详情 |
| `Ctrl+B` | 切换书签 |
| `Ctrl+R` | 重放请求 |

#### 验收标准

- [ ] 所有快捷键正确响应
- [ ] 快捷键不与系统/浏览器快捷键冲突
- [ ] 输入框内快捷键不触发
- [ ] 快捷键列表可在设置中查看

---

## 步骤七：创新功能与差异化竞争

> **优先级**: 🟢 P2  
> **预计工期**: 8 周（可并行开发）  
> **目标**: 实现超越 HTTP Debugger Pro 的创新功能

### 7.1 子步骤：AI 智能分析助手（2周）

#### 新增文件

```
src/components/ai/
├── AiAssistant.tsx
├── AiChatInterface.tsx
└── AiInsights.tsx
crates/flowreveal-app/src/commands/
└── ai.rs
```

#### 功能

- 自然语言查询流量
- 异常检测与性能瓶颈分析
- 安全审计（敏感信息泄露检测）
- API 文档自动生成

#### 验收标准

- [ ] 可用自然语言查询："找出所有超过2秒的API调用"
- [ ] 自动检测异常流量模式
- [ ] AI 分析结果可操作（点击跳转到对应请求）

---

### 7.2 子步骤：流量 Diff 对比工具（1.5周）

#### 新增文件

```
src/components/tools/
└── TrafficDiff.tsx
```

#### 验收标准

- [ ] 可选择两次捕获进行对比
- [ ] 差异部分高亮显示
- [ ] 生成变更报告

---

### 7.3 子步骤：GraphQL 专用解析器（1周）

#### 新增文件

```
crates/engine-core/src/protocol/
└── graphql.rs
src/components/detail/
└── GraphQLView.tsx
```

#### 验收标准

- [ ] 自动识别 GraphQL 请求
- [ ] 解析 Query/Mutation/Subscription
- [ ] 变量格式化展示
- [ ] 语法高亮

---

### 7.4 子步骤：gRPC-Web 支持（1.5周）

#### 新增文件

```
crates/engine-core/src/protocol/
└── grpc_web.rs
src/components/detail/
└── GrpcView.tsx
```

#### 验收标准

- [ ] 识别 gRPC-Web 请求
- [ ] Protobuf 消息解码（提供 .proto 时）
- [ ] Service/Method 路径解析

---

### 7.5 子步骤：自动漏洞扫描（2周）

#### 功能

| 漏洞类型 | 检测方式 |
|---------|---------|
| SQL注入 | 参数中SQL关键字 |
| XSS | 反射型XSS模式 |
| 敏感信息泄露 | 响应中的密码/Token |
| CORS配置错误 | 过于宽松的CORS头 |
| 缺少安全头 | X-Frame-Options等 |

#### 验收标准

- [ ] 扫描结果按严重程度分级
- [ ] 每个漏洞关联到具体请求
- [ ] 提供修复建议

---

### 7.6 子步骤：插件系统（2周）

#### 新增目录

```
plugins/
└── example-plugin/
    ├── manifest.json
    └── main.js
crates/engine-core/src/plugin/
├── mod.rs
├── plugin_manager.rs
└── plugin_api.rs
```

#### 验收标准

- [ ] 插件可自定义规则、列、详情面板
- [ ] 插件沙箱隔离
- [ ] 至少1个示例插件可运行

---

## 9. 风险与依赖关系

### 9.1 步骤间依赖

```
步骤一（规则系统）────┐
                      ├──→ 步骤四（编辑工具，右键菜单依赖规则系统）
步骤二（过滤搜索）────┤
                      ├──→ 步骤六（性能优化，需在功能稳定后优化）
步骤三（统计可视化）──┤
                      ├──→ 步骤七（创新功能，可独立并行）
步骤五（协议增强）────┘
```

**关键路径**：步骤一 → 步骤四 → 步骤六

### 9.2 技术风险

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| 规则引擎性能瓶颈 | 高 | 异步执行、规则预编译、缓存匹配结果 |
| HTTP/2 实现复杂度 | 中 | 使用成熟的 `h2` crate，充分测试 |
| 全文搜索内存占用 | 中 | 增量索引、限制索引字段、LRU淘汰 |
| 规则与代理集成冲突 | 高 | 分阶段集成，每阶段充分测试 |

---

## 10. 里程碑与交付时间线

### 10.1 里程碑定义

| 里程碑 | 包含步骤 | 交付物 | 预计完成 |
|--------|---------|--------|---------|
| **M1: 规则系统可用** | 步骤一 | 自动回复/修改头/重定向 + 规则管理UI | 第4周 |
| **M2: 过滤搜索完善** | 步骤二 | DSL过滤 + 全文搜索 | 第6周 |
| **M3: 统计可视化** | 步骤三 | 性能统计面板 | 第7.5周 |
| **M4: 编辑工具增强** | 步骤四 | 数据转换器/书签/右键菜单 | 第9周 |
| **M5: 性能优化** | 步骤六 | 虚拟滚动/分页/主题/快捷键 | 第11周 |
| **M6: 协议全覆盖** | 步骤五 | HTTP/2 + WebSocket | 第14.5周 |
| **M7: 创新功能** | 步骤七 | AI/Diff/GraphQL/gRPC/漏洞扫描/插件 | 第22.5周 |

### 10.2 关键检查点

| 检查点 | 日期 | 检查内容 |
|--------|------|---------|
| CP1 | 第2周末 | 规则数据结构+匹配引擎编译通过+单元测试 |
| CP2 | 第4周末 | 规则系统端到端可用（UI+后端+代理集成） |
| CP3 | 第6周末 | DSL过滤+搜索功能完整可用 |
| CP4 | 第9周末 | P0+P1功能全部完成，可进行内部测试 |
| CP5 | 第11周末 | 性能指标达标（万级数据流畅、内存<200MB） |
| CP6 | 第14.5周末 | **全面超越 HTTP Debugger Pro** 🎉 |
| CP7 | 第22.5周末 | 创新功能发布，行业领先 |

### 10.3 每步完成后的回归测试清单

每个子步骤完成后，执行以下回归测试：

- [ ] `cargo build` 编译通过
- [ ] `cargo test` 所有测试通过
- [ ] 启动应用，开始/停止抓包正常
- [ ] HTTP 请求正确捕获和显示
- [ ] HTTPS 请求正确解密和显示
- [ ] 请求详情面板正确展示
- [ ] HAR 导出功能正常
- [ ] 请求重放功能正常
- [ ] 无内存泄漏（长时间运行内存稳定）
- [ ] 无崩溃（连续操作100次）
