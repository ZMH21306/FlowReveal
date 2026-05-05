# FlowReveal 全局流量捕获开发规划 (DEVPLAN2)

> 目标：实现与 HTTP Debugger Pro 同等的全局流量捕获能力，能够拦截系统中所有进程（含 SYSTEM/Service/非代理应用）的 HTTP/HTTPS 流量。

---

## 一、现状诊断

### 当前架构缺陷

| 缺陷 | 影响 | 严重度 |
|------|------|--------|
| 透明代理仅绑定 127.0.0.1，无 WFP 重定向 | 非 proxy-aware 应用流量完全不可见 | 致命 |
| 透明代理收到连接后只记录不转发 | 即使流量到达也无法正常通信 | 致命 |
| 透明代理无 MITM/HTTPS 解密 | HTTPS 流量只能看到隧道 | 严重 |
| 正向代理仅监听 IPv4 | IPv6 流量丢失 | 中等 |
| 无 localhost 流量捕获 | 本地服务间通信不可见 | 中等 |
| 进程查找基于连接快照，时序竞争 | 部分短连接进程信息丢失 | 低 |

### 对比 HTTP Debugger Pro 的差距

HTTP Debugger Pro 使用 **Winsock LSP / 内核驱动** 在网络协议栈层面全局拦截，而 FlowReveal 当前仅在用户态作为 HTTP 代理运行，只能捕获主动配置使用该代理的应用（主要是浏览器）。

---

## 二、技术方案选型

### 方案对比

| 方案 | 原理 | 优势 | 劣势 | 可行性 |
|------|------|------|------|--------|
| **A. WinDivert** | 用户态 NDIS 驱动拦截/修改/重注入数据包 | 无需签名驱动、Rust crate 成熟、支持 localhost/IPv6 | 需管理员权限、Wi-Fi 可能存在 fast-path 问题 | ★★★★ |
| **B. WFP callout 驱动** | 内核态 WFP 分类/重定向 | 最底层拦截、无 Wi-Fi 限制 | 需签名内核驱动、开发/调试极难、分发受限 | ★★ |
| **C. Winsock LSP** | Winsock 分层服务提供者 | 兼容性好 | 已被微软标记为过时、安装复杂、64位受限 | ★ |
| **D. WinDivert + WFP 混合** | WinDivert 做数据包重定向 + WFP 做连接分类 | 兼顾覆盖面和稳定性 | 架构复杂 | ★★★ |

### 决策：采用方案 A — WinDivert

理由：
1. `windivert` Rust crate 已成熟（docs.rs/windivert），FFI 封装完善
2. WinDivert 2.2 支持 NETWORK 层拦截，支持 localhost 和 IPv6
3. 无需内核驱动签名，以管理员权限运行即可
4. mitmproxy 的 Windows 版本也使用 WinDivert 做透明代理
5. 社区广泛使用（GoodbyeDPI、Suricata、clumsy 等）

**Wi-Fi fast-path 风险缓解**：检测 Wi-Fi 适配器时自动回退到系统代理模式，并在 UI 中提示用户。

---

## 三、架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────┐
│                    应用层 (所有进程)                       │
│  浏览器 / ClassIn / Steam / svchost / Autodesk / ...     │
└──────────────┬──────────────────────────────────────────┘
               │ 所有出站 TCP (80/443/自定义端口)
               ▼
┌─────────────────────────────────────────────────────────┐
│              WinDivert NDIS 驱动层                        │
│  拦截: outbound tcp.DstPort == 80 or 443                 │
│  拦截: inbound from local proxy (回程 NAT 还原)           │
│  操作: DNAT 目标 → 127.0.0.1:LOCAL_PORT                  │
│        SNAT 源 → 127.0.0.1 (伪装来源)                     │
│        记录原始 (dst_ip, dst_port) 映射                    │
└──────────────┬──────────────────────────────────────────┘
               │ 重定向后的连接
               ▼
┌─────────────────────────────────────────────────────────┐
│            FlowReveal 本地透明代理引擎                     │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ HTTP 明文    │  │ HTTPS MITM   │  │ 隧道回退      │  │
│  │ 解析+转发    │  │ TLS 拦截+解密│  │ 不解密仅记录  │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
│  进程识别: GetExtendedTcpTable + WFP 连接上下文           │
│  规则引擎: 自动回复/头部修改/重定向                        │
└──────────────┬──────────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────┐
│              前端展示 (React/Tauri)                       │
│  流量列表 / 请求详情 / 规则面板 / 统计 / 工具             │
└─────────────────────────────────────────────────────────┘
```

### 数据包重定向流程 (DNAT/SNAT)

```
原始出站: App(192.168.1.5:12345) → Server(93.184.216.34:443)
   ↓ WinDivert 拦截
修改出站: App(192.168.1.5:12345) → LocalProxy(127.0.0.1:8899)
   ↓ 记录映射: (192.168.1.5:12345) → (93.184.216.34:443)
   ↓ 注入回网络栈
   ↓ Windows 完成 TCP 握手 → LocalProxy 收到连接
   ↓ LocalProxy 从映射表查到原始目标
   ↓ LocalProxy 连接 Server(93.184.216.34:443)

回程入站: LocalProxy(127.0.0.1:8899) → App(192.168.1.5:12345)
   ↓ WinDivert 拦截
还原入站: Server(93.184.216.34:443) → App(192.168.1.5:12345)
   ↓ 注入回网络栈
   ↓ App 认为在与原始服务器通信
```

---

## 四、开发步骤

### 步骤 1：WinDivert 集成与数据包重定向引擎

**目标**：实现 WinDivert 数据包拦截 + DNAT/SNAT 重定向，让所有出站 TCP 80/443 流量到达本地透明代理。

#### 1.1 添加 WinDivert 依赖

- 在 `engine-core/Cargo.toml` 添加 `windivert = "0.10"` 依赖
- 捆绑 WinDivert 2.2 DLL (`WinDivert.dll` + `WinDivert64.sys`) 到应用资源目录
- 运行时自动解压 DLL 到临时目录并加载

#### 1.2 实现数据包重定向器 `PacketDiverter`

新建 `crates/engine-core/src/divert/` 模块：

```
divert/
├── mod.rs              # 模块注册
├── diverter.rs         # PacketDiverter 主结构
├── nat_table.rs        # NAT 映射表 (连接追踪)
├── packet_processor.rs # 数据包解析与修改
└── wifi_detect.rs      # Wi-Fi 适配器检测
```

**`diverter.rs` 核心逻辑**：

```rust
pub struct PacketDiverter {
    proxy_port: u16,           // 本地透明代理端口
    exclude_pids: HashSet<u32>, // 排除自身进程
    nat_table: NatTable,       // 原始目标映射表
    handle: Option<WinDivert>,  // WinDivert 句柄
}
```

- WinDivert 过滤器：`(outbound and tcp.DstPort == 80 or tcp.DstPort == 443) or (inbound and tcp.SrcPort == {proxy_port})`
- 出站包：DNAT 修改目标为 `127.0.0.1:proxy_port`，记录原始 `(src, dst)` 到 NAT 表
- 入站包：从 NAT 表查找还原原始源地址
- 排除自身进程：通过 `WinDivert` address 的 `ProcessId` 排除 FlowReveal 自身
- Wi-Fi 检测：启动前检测主网卡类型，Wi-Fi 时记录警告

**`nat_table.rs` 核心逻辑**：

```rust
pub struct NatTable {
    // key: (client_ip, client_port) → value: (original_dst_ip, original_dst_port)
    outbound: DashMap<(Ipv4Addr, u16), (Ipv4Addr, u16)>,
    // key: (client_ip, client_port) → value: (original_dst_ip, original_dst_port)
    // 用于入站还原
    inbound: DashMap<(Ipv4Addr, u16), (Ipv4Addr, u16)>,
}
```

- 使用 `DashMap` 并发安全
- TCP FIN/RST 时自动清理映射
- 超时清理：5 分钟无活动的映射自动删除

**`packet_processor.rs`**：

- 解析 IP/TCP 头部
- 修改目标/源地址后重算 IP 和 TCP 校验和
- 使用 `WinDivertHelperCalcChecksums` 辅助计算

**`wifi_detect.rs`**：

- 通过 `GetAdaptersInfo` / `GetAdaptersAddresses` 检测主网卡类型
- 如果是 Wi-Fi (IF_TYPE_IEEE80211 = 71)，记录警告
- 提供 `is_wifi_adapter() -> bool` 供 UI 查询

#### 1.3 IPv6 支持

- WinDivert 同时拦截 IPv6 数据包
- NAT 表增加 IPv6 条目
- 透明代理同时监听 IPv6 (`[::1]:proxy_port`)

#### 1.4 localhost 流量捕获

- WinDivert 支持 loopback 流量拦截（设置 `WINDIVERT_LAYER_NETWORK` + loopback flag）
- 过滤器增加 `loopback and tcp.DstPort == 80 or tcp.DstPort == 443`
- 透明代理需要处理来自 `127.0.0.1` 的连接

---

### 步骤 2：透明代理引擎重写

**目标**：将当前的空壳透明代理重写为完整的透明代理引擎，支持 HTTP 明文解析、HTTPS MITM 解密、隧道回退。

#### 2.1 重写 `transparent_proxy.rs`

当前问题：
- 只绑定 `127.0.0.1`（需同时绑定 `[::1]`）
- 收到连接后只返回 `200 OK` 不做转发
- 无 MITM 能力
- 无进程识别

新架构：

```rust
pub struct TransparentProxy {
    proxy_port: u16,
    ca_manager: Option<Arc<CaManager>>,
    nat_table: Arc<NatTable>,  // 从 PacketDiverter 共享
    rule_engine: Arc<RuleEngine>,
}
```

核心流程：
1. 收到新连接 → 从 NAT 表查找原始目标 `(dst_ip, dst_port)`
2. 根据原始端口判断协议：
   - 端口 80 → HTTP 明文处理
   - 端口 443 → HTTPS MITM 处理
   - 其他端口 → 隧道回退
3. 进程识别：通过 `GetExtendedTcpTable` 查找连接对应的 PID
4. 规则引擎应用

#### 2.2 HTTP 明文透明代理

```
Client → [WinDivert DNAT] → TransparentProxy:8899
   ↓ 读取 HTTP 请求
   ↓ 从 NAT 表获取原始目标
   ↓ 连接原始目标服务器
   ↓ 转发请求 + 记录响应
   ↓ 返回响应给客户端
Client ← [WinDivert SNAT 还原] ← TransparentProxy
```

- 复用现有 `forward_proxy.rs` 的 HTTP 转发逻辑
- 关键区别：请求行中的 URL 是相对路径（透明代理模式），需用 NAT 表中的原始目标构造完整 URL

#### 2.3 HTTPS MITM 透明代理

```
Client → [WinDivert DNAT] → TransparentProxy:8899
   ↓ 直接进行 TLS Server Hello（客户端以为在连原始服务器）
   ↓ 使用 CaManager 为原始域名签发证书
   ↓ 完成与客户端的 TLS 握手
   ↓ 读取解密后的 HTTP 请求
   ↓ 连接原始目标服务器（TLS）
   ↓ 转发请求 + 记录响应
   ↓ 加密响应返回客户端
Client ← [WinDivert SNAT 还原] ← TransparentProxy
```

- **关键区别**：透明代理模式下，客户端不知道自己在与代理通信，不会发送 CONNECT 请求
- 客户端直接发送 TLS Client Hello，代理需要立即以 TLS Server 身份响应
- 使用 SNI (Server Name Indication) 从 Client Hello 中提取目标域名
- 如果 SNI 缺失，使用 NAT 表中的 IP 地址做反向 DNS 或直接用 IP 签发证书

#### 2.4 SNI 提取器

新建 `crates/engine-core/src/divert/sni_parser.rs`：

- 解析 TLS Client Hello 消息
- 提取 SNI 扩展中的域名
- 用于在无 CONNECT 请求时确定 MITM 目标域名

#### 2.5 隧道回退模式

- 对于非 HTTP/HTTPS 端口（如 SMTP、IMAP、自定义协议）
- 不做协议解析，仅做 TCP 层面的双向转发
- 记录连接元数据（源/目标 IP:Port、进程、时间、字节数）

---

### 步骤 3：进程识别增强

**目标**：准确识别每个连接的发起进程，包括 SYSTEM/Service 上下文。

#### 3.1 WFP 连接上下文

- 在 WinDivert 拦截数据包时，`WINDIVERT_ADDRESS` 结构包含 `ProcessId`
- 直接从数据包元数据获取 PID，无需事后查表
- 解决当前 `GetExtendedTcpTable` 时序竞争问题

#### 3.2 进程信息丰富化

- PID → 进程名/路径（已有 `get_process_info`）
- 增加：用户名（通过 `OpenProcessToken` + `GetTokenInformation`）
- 增加：是否 64 位进程（通过 `IsWow64Process`）
- 增加：进程图标提取（可选，用于 UI 展示）

#### 3.3 进程过滤

- 前端增加进程过滤面板：按进程名/用户/PID 过滤
- 后端 `CaptureConfig` 增加 `include_pids` / `exclude_pids` 配置
- WinDivert 过滤器中直接排除指定 PID，减少不必要的数据包处理

---

### 步骤 4：捕获模式重构

**目标**：简化捕获模式，统一为"全局捕获"模式。

#### 4.1 捕获模式重新定义

| 模式 | 说明 | WinDivert | 系统代理 |
|------|------|-----------|----------|
| 全局捕获 | 拦截所有进程流量 | ✅ 启动 | ✅ 启动（浏览器兼容） |
| 仅代理 | 仅捕获使用代理的应用 | ❌ 不启动 | ✅ 启动 |

- 默认使用"全局捕获"模式
- 如果 WinDivert 加载失败（非管理员/驱动冲突），自动回退到"仅代理"模式
- 前端 UI 更新：移除旧的模式选择，改为开关式"全局捕获"

#### 4.2 CaptureConfig 更新

```rust
pub struct CaptureConfig {
    pub mode: CaptureMode,           // Global / ProxyOnly
    pub proxy_port: u16,             // 正向代理端口
    pub transparent_proxy_port: u16, // 透明代理端口
    pub capture_https: bool,
    pub max_body_size: usize,
    pub ca_cert_path: Option<String>,
    pub ca_key_path: Option<String>,
    pub mitm_bypass_hosts: Vec<String>,
    pub capture_ports: Vec<u16>,     // 新增：自定义捕获端口
    pub exclude_pids: Vec<u32>,      // 新增：排除的进程
    pub capture_localhost: bool,     // 新增：是否捕获 localhost
}
```

#### 4.3 启动流程重构

```
start_capture(config):
  1. 初始化 CA 管理器（如果 capture_https）
  2. 安装 CA 证书到系统
  3. 启动正向代理（proxy_port）— 处理浏览器等 proxy-aware 应用
  4. 启动透明代理（transparent_proxy_port）— 处理 WinDivert 重定向的流量
  5. 启动 WinDivert 数据包重定向器 — DNAT/SNAT
  6. 设置系统代理
  7. 启动流量处理线程
```

停止流程：
```
stop_capture():
  1. 停止 WinDivert 数据包重定向器
  2. 停止透明代理
  3. 停止正向代理
  4. 恢复系统代理
  5. 卸载 CA 证书
```

---

### 步骤 5：前端 UI 适配

**目标**：前端适配全局捕获模式，增加进程过滤和状态提示。

#### 5.1 工具栏更新

- 移除旧的捕获模式下拉框（已完成）
- 增加"全局捕获"开关（需要管理员权限提示）
- 增加捕获端口配置（默认 80, 443）
- 增加"捕获 localhost"开关
- 显示 WinDivert 状态指示器

#### 5.2 进程列显示

- 流量列表增加"进程"列
- 进程名 + PID 显示
- 支持按进程名过滤

#### 5.3 状态栏增强

- 显示：捕获模式 / 活动连接数 / WinDivert 状态
- Wi-Fi 警告提示（如果检测到 Wi-Fi 适配器）

#### 5.4 权限提示

- 全局捕获需要管理员权限
- 启动时如果非管理员，提示用户以管理员身份重新运行
- 使用 `ShellExecuteW` 的 `runas` 动词请求提权

---

### 步骤 6：稳定性与性能

**目标**：确保全局捕获模式下的稳定性和性能。

#### 6.1 数据包处理性能

- WinDivert 接收循环使用专用线程，避免阻塞 tokio 运行时
- 数据包修改后立即重注入，减少延迟
- NAT 表使用 `DashMap` 无锁并发读取
- 批量接收：使用 `WinDivertRecvEx` 批量接收数据包

#### 6.2 连接追踪可靠性

- TCP 状态机追踪：SYN → ESTABLISHED → FIN/RST
- 超时清理：5 分钟无活动自动清理 NAT 映射
- 异常恢复：WinDivert 句柄丢失时自动重建

#### 6.3 资源管理

- WinDivert DLL 运行时动态加载，避免非 Windows 平台编译错误
- 停止捕获时确保所有 WinDivert 句柄正确关闭
- 停止捕获时确保 NAT 表清空
- 内存使用监控：NAT 表条目上限（默认 65536）

#### 6.4 错误恢复

- WinDivert 驱动加载失败 → 回退到仅代理模式
- 透明代理端口冲突 → 自动尝试下一个端口
- CA 证书安装失败 → 降级为仅 HTTP 捕获

---

### 步骤 7：测试与验证

#### 7.1 单元测试

- NAT 表 CRUD 操作
- 数据包解析与修改（IP/TCP 头部）
- SNI 提取器
- 校验和计算

#### 7.2 集成测试

- 全局捕获模式：启动 WinDivert + 透明代理 + 正向代理
- 测试场景：
  - 浏览器 HTTP/HTTPS 访问
  - 非 proxy-aware 应用（curl --noproxy, PowerShell Invoke-WebRequest）
  - 系统服务流量（svchost OCSP）
  - 本地服务间通信（localhost）
  - IPv6 连接
  - WebSocket 升级
  - 大文件传输（性能）

#### 7.3 对比验证

- 同时运行 HTTP Debugger Pro 和 FlowReveal
- 比较捕获的会话数量和内容
- 验证进程识别准确性
- 验证 HTTPS 解密完整性

---

## 五、文件变更清单

### 新增文件

| 文件路径 | 说明 |
|----------|------|
| `crates/engine-core/src/divert/mod.rs` | WinDivert 模块注册 |
| `crates/engine-core/src/divert/diverter.rs` | 数据包重定向器 |
| `crates/engine-core/src/divert/nat_table.rs` | NAT 映射表 |
| `crates/engine-core/src/divert/packet_processor.rs` | 数据包解析与修改 |
| `crates/engine-core/src/divert/sni_parser.rs` | TLS SNI 提取 |
| `crates/engine-core/src/divert/wifi_detect.rs` | Wi-Fi 适配器检测 |
| `crates/engine-core/src/divert/elevation.rs` | 管理员权限检查与提权 |
| `resources/WinDivert.dll` | WinDivert 运行时 DLL (x86) |
| `resources/WinDivert64.sys` | WinDivert 内核驱动 (x64) |

### 重写文件

| 文件路径 | 说明 |
|----------|------|
| `crates/engine-core/src/proxy/transparent_proxy.rs` | 完整重写：HTTP/HTTPS/隧道 |
| `crates/engine-core/src/capture_config.rs` | 新增字段：capture_ports/exclude_pids/capture_localhost |

### 修改文件

| 文件路径 | 说明 |
|----------|------|
| `crates/engine-core/src/lib.rs` | 注册 divert 模块 |
| `crates/engine-core/src/proxy/mod.rs` | 导出新的透明代理 |
| `crates/engine-core/src/proxy/utils.rs` | 新增 SNI 相关工具函数 |
| `crates/engine-core/src/platform_integration/windows.rs` | 新增管理员权限检查/提权 |
| `crates/engine-core/Cargo.toml` | 添加 windivert 依赖 |
| `crates/flowreveal-app/src/commands/capture.rs` | 重构启动/停止流程 |
| `crates/flowreveal-app/src/state.rs` | 新增 WinDivert 状态 |
| `src/types/index.ts` | 更新 CaptureConfig 类型 |
| `src/components/layout/Toolbar.tsx` | 全局捕获开关 |
| `src/components/traffic/TrafficList.tsx` | 进程列 |
| `src/components/layout/StatusBar.tsx` | WinDivert 状态 |

---

## 六、开发优先级与依赖关系

```
步骤 1: WinDivert 集成 ──────────────────┐
                                         │
步骤 2: 透明代理重写 ────────────────────┤ (依赖步骤 1)
                                         │
步骤 3: 进程识别增强 ────────────────────┤ (依赖步骤 1)
                                         │
步骤 4: 捕获模式重构 ────────────────────┤ (依赖步骤 1+2)
                                         │
步骤 5: 前端 UI 适配 ────────────────────┤ (依赖步骤 4)
                                         │
步骤 6: 稳定性与性能 ────────────────────┤ (依赖步骤 1-5)
                                         │
步骤 7: 测试与验证 ──────────────────────┘ (依赖步骤 1-6)
```

---

## 七、风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| Wi-Fi fast-path 导致出站包不可见 | 中 | 高 | 检测 Wi-Fi 并提示；回退到系统代理模式 |
| WinDivert 驱动被杀毒软件拦截 | 低 | 高 | 提供手动安装指引；考虑自签名驱动 |
| 管理员权限获取失败 | 中 | 中 | 回退到仅代理模式；UI 提示 |
| NAT 表内存溢出 | 低 | 中 | 条目上限 + LRU 淘汰 |
| 透明代理 HTTPS 解密失败 | 中 | 中 | 回退到隧道模式；记录原始连接元数据 |
| WinDivert DLL 加载失败 | 低 | 高 | 运行时动态加载；编译期 cfg 条件 |
