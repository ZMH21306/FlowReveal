# FlowReveal 全局流量捕获 — 开发检查清单

> 对照 DEVPLAN2.md 的每个步骤，逐项验收开发完成度。

---

## 步骤 1：WinDivert 集成与数据包重定向引擎

### 1.1 依赖与构建

- [ ] `engine-core/Cargo.toml` 添加 `windivert` 依赖
- [ ] WinDivert DLL (x86) 捆绑到 `resources/` 目录
- [ ] WinDivert 驱动 (x64) 捆绑到 `resources/` 目录
- [ ] 运行时动态加载 WinDivert DLL（非编译期链接）
- [ ] 非 Windows 平台编译不受影响（cfg 条件编译）

### 1.2 PacketDiverter 核心模块

- [ ] `divert/mod.rs` 模块注册完成
- [ ] `divert/diverter.rs` — `PacketDiverter` 结构体定义
- [ ] WinDivert 句柄创建与过滤器配置
- [ ] 出站数据包拦截（outbound tcp.DstPort == 80/443）
- [ ] 入站数据包拦截（inbound tcp.SrcPort == proxy_port）
- [ ] DNAT：出站包目标地址改写为 `127.0.0.1:proxy_port`
- [ ] SNAT：入站包源地址还原为原始服务器地址
- [ ] 排除自身进程（通过 WinDivert address.ProcessId）
- [ ] TCP FIN/RST 检测与 NAT 表清理
- [ ] WinDivert 句柄正确关闭（stop 时）

### 1.3 NAT 映射表

- [ ] `divert/nat_table.rs` — `NatTable` 结构体
- [ ] 出站映射记录：`(client_ip, client_port) → (original_dst_ip, original_dst_port)`
- [ ] 入站映射查找与还原
- [ ] DashMap 并发安全读写
- [ ] 超时清理（5 分钟无活动）
- [ ] 条目上限（65536）+ LRU 淘汰
- [ ] TCP FIN/RST 触发即时清理
- [ ] `get_original_dest(client_ip, client_port) -> Option<(IpAddr, u16)>` 接口

### 1.4 数据包处理

- [ ] `divert/packet_processor.rs` — IP/TCP 头部解析
- [ ] IPv4 头部解析与修改
- [ ] IPv6 头部解析与修改
- [ ] TCP 头部解析与修改
- [ ] IP 校验和重算
- [ ] TCP 校验和重算（使用 `WinDivertHelperCalcChecksums`）
- [ ] 数据包重注入（`WinDivertSend`）

### 1.5 SNI 提取器

- [ ] `divert/sni_parser.rs` — TLS Client Hello 解析
- [ ] 提取 SNI 扩展中的域名
- [ ] 处理无 SNI 的情况（回退到 IP 地址）
- [ ] 处理畸形 Client Hello（不崩溃）

### 1.6 Wi-Fi 检测

- [ ] `divert/wifi_detect.rs` — 适配器类型检测
- [ ] `is_wifi_adapter() -> bool` 接口
- [ ] 检测到 Wi-Fi 时记录警告日志
- [ ] 提供适配器信息供前端展示

### 1.7 管理员权限

- [ ] `divert/elevation.rs` — 权限检查
- [ ] `is_elevated() -> bool` 检查当前是否管理员
- [ ] `request_elevation() -> Result<()>` 请求 UAC 提权
- [ ] 非管理员时自动回退到仅代理模式

### 1.8 IPv6 支持

- [ ] WinDivert 过滤器包含 IPv6 规则
- [ ] NAT 表支持 IPv6 地址
- [ ] 透明代理监听 `[::1]:proxy_port`
- [ ] IPv6 数据包 DNAT/SNAT 处理

### 1.9 localhost 流量捕获

- [ ] WinDivert 过滤器包含 loopback 规则
- [ ] 透明代理处理来自 `127.0.0.1` 的连接
- [ ] localhost 连接的 NAT 映射正确
- [ ] 可通过 `capture_localhost` 配置开关

---

## 步骤 2：透明代理引擎重写

### 2.1 核心重写

- [ ] `transparent_proxy.rs` 完整重写
- [ ] 同时监听 IPv4 (`127.0.0.1`) 和 IPv6 (`[::1]`)
- [ ] 收到连接后从 NAT 表查询原始目标
- [ ] 根据原始端口分发：80→HTTP / 443→HTTPS / 其他→隧道
- [ ] 集成 `CaManager`（HTTPS MITM）
- [ ] 集成 `RuleEngine`（规则应用）
- [ ] 进程识别（从 WinDivert address 或 GetExtendedTcpTable）

### 2.2 HTTP 明文透明代理

- [ ] 读取 HTTP 请求（相对路径 URL）
- [ ] 从 NAT 表构造完整 URL
- [ ] 连接原始目标服务器
- [ ] 转发请求并记录
- [ ] 接收响应并记录
- [ ] 返回响应给客户端
- [ ] 规则引擎应用（自动回复/头部修改/重定向）

### 2.3 HTTPS MITM 透明代理

- [ ] 接受连接后直接进行 TLS Server Hello
- [ ] 从 SNI 或 NAT 表获取目标域名
- [ ] 使用 CaManager 签发域名证书
- [ ] 完成与客户端的 TLS 握手
- [ ] 读取解密后的 HTTP 请求
- [ ] 连接原始目标服务器（TLS）
- [ ] 转发请求并记录
- [ ] 接收响应并记录
- [ ] 加密响应返回客户端
- [ ] TLS 版本和密码套件信息记录
- [ ] 规则引擎应用

### 2.4 隧道回退模式

- [ ] 非 HTTP/HTTPS 端口的 TCP 双向转发
- [ ] 记录连接元数据（IP:Port、进程、时间、字节数）
- [ ] 不做协议解析，仅透传

---

## 步骤 3：进程识别增强

### 3.1 WinDivert 进程 ID

- [ ] 从 `WINDIVERT_ADDRESS.ProcessId` 获取 PID
- [ ] PID 传递到 HttpMessage 的 process_info
- [ ] 解决 GetExtendedTcpTable 时序竞争问题

### 3.2 进程信息丰富化

- [ ] 用户名获取（OpenProcessToken + GetTokenInformation）
- [ ] 是否 64 位进程（IsWow64Process）
- [ ] ProcessInfo 结构体增加 user / is_64bit 字段

### 3.3 进程过滤

- [ ] CaptureConfig 增加 include_pids / exclude_pids
- [ ] WinDivert 过滤器中排除指定 PID
- [ ] 前端进程过滤面板
- [ ] 按进程名 / 用户 / PID 过滤

---

## 步骤 4：捕获模式重构

### 4.1 模式定义

- [ ] CaptureMode 枚举更新：Global / ProxyOnly
- [ ] 前端类型定义更新
- [ ] 默认模式为 Global

### 4.2 CaptureConfig 更新

- [ ] 新增 `capture_ports: Vec<u16>` 字段
- [ ] 新增 `exclude_pids: Vec<u32>` 字段
- [ ] 新增 `capture_localhost: bool` 字段
- [ ] 前端 CaptureConfig 类型同步更新

### 4.3 启动流程

- [ ] start_capture 重构：先启动代理，再启动 WinDivert
- [ ] WinDivert 加载失败时自动回退到 ProxyOnly
- [ ] 停止流程：先停 WinDivert，再停代理
- [ ] 状态管理：AppState 增加 WinDivert 状态

---

## 步骤 5：前端 UI 适配

### 5.1 工具栏

- [ ] "全局捕获"开关（需要管理员权限提示）
- [ ] 捕获端口配置（默认 80, 443）
- [ ] "捕获 localhost"开关
- [ ] WinDivert 状态指示器

### 5.2 流量列表

- [ ] 增加"进程"列（进程名 + PID）
- [ ] 进程名列支持点击过滤
- [ ] 进程图标（可选）

### 5.3 状态栏

- [ ] 捕获模式显示
- [ ] WinDivert 状态显示
- [ ] Wi-Fi 警告提示
- [ ] 管理员权限状态

### 5.4 权限提示

- [ ] 非管理员时显示提权对话框
- [ ] UAC 提权请求（ShellExecuteW runas）
- [ ] 提权后自动重新启动捕获

---

## 步骤 6：稳定性与性能

### 6.1 数据包处理性能

- [ ] WinDivert 接收循环使用专用线程
- [ ] 批量接收（WinDivertRecvEx）
- [ ] 数据包修改后立即重注入
- [ ] NAT 表 DashMap 无锁读取

### 6.2 连接追踪可靠性

- [ ] TCP 状态机追踪（SYN/ESTABLISHED/FIN/RST）
- [ ] 超时清理定时器
- [ ] WinDivert 句柄丢失时自动重建
- [ ] 异常断开时 NAT 表清理

### 6.3 资源管理

- [ ] WinDivert DLL 运行时动态加载
- [ ] 停止捕获时所有句柄正确关闭
- [ ] 停止捕获时 NAT 表清空
- [ ] 内存使用监控

### 6.4 错误恢复

- [ ] WinDivert 驱动加载失败 → 回退仅代理模式
- [ ] 透明代理端口冲突 → 自动尝试下一端口
- [ ] CA 证书安装失败 → 降级仅 HTTP
- [ ] 所有回退都有 UI 提示

---

## 步骤 7：测试与验证

### 7.1 单元测试

- [ ] NAT 表 CRUD 操作测试
- [ ] 数据包解析与修改测试
- [ ] SNI 提取器测试
- [ ] 校验和计算测试
- [ ] Wi-Fi 检测测试

### 7.2 集成测试

- [ ] 全局捕获模式启动/停止
- [ ] 浏览器 HTTP/HTTPS 访问捕获
- [ ] 非 proxy-aware 应用捕获（curl --noproxy）
- [ ] 系统服务流量捕获（svchost OCSP）
- [ ] 本地服务间通信捕获（localhost）
- [ ] IPv6 连接捕获
- [ ] WebSocket 升级捕获
- [ ] 大文件传输性能测试
- [ ] 高并发连接稳定性测试

### 7.3 对比验证

- [ ] 同时运行 HTTP Debugger Pro 和 FlowReveal
- [ ] 比较捕获的会话数量
- [ ] 验证进程识别准确性
- [ ] 验证 HTTPS 解密完整性
- [ ] 验证 ClassIn / Steam 等应用流量可见

---

## 编译与运行验证

- [ ] `cargo check` 通过
- [ ] `cargo test` 通过
- [ ] `npx tsc --noEmit` 通过
- [ ] `npx tauri dev` 启动成功
- [ ] 全局捕获模式实际运行测试
