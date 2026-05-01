# FlowReveal — HTTP 调试器开发总体规划

---

## 目录

1. [项目整体架构设计](#1-项目整体架构设计)
2. [关键技术点解决方案](#2-关键技术点解决方案)
3. [详细开发路线图（6 阶段）](#3-详细开发路线图)
4. [代码组织与模块依赖](#4-代码组织与模块依赖)
5. [潜在风险与缓解措施](#5-潜在风险与缓解措施)

---

## 1. 项目整体架构设计

### 1.1 分层架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                    UI Layer (React + TypeScript)                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │Dashboard │ │Traffic   │ │Request   │ │Settings  │           │
│  │Panel     │ │List View │ │Detail    │ │Panel     │           │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
│                         ▲                                        │
│                    Tauri IPC (invoke/event)                       │
├───────────────────────┼──────────────────────────────────────────┤
│              Bridge Layer (Rust — Tauri Commands)                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  IpcHandlers: start_capture, stop_capture, get_requests, │    │
│  │  export_har, install_cert, replay_request, ...          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                         ▲                                        │
│                  Trait: InterceptEngine                          │
├───────────────────────┼──────────────────────────────────────────┤
│                   Core Engine Layer (Rust)                       │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐       │
│  │ MITM      │ │ Protocol  │ │ Traffic   │ │ Process   │       │
│  │ Engine    │ │ Parser    │ │ Capture   │ │ Resolver  │       │
│  │(cert_     │ │(http1/2/  │ │(wfp_,     │ │(pid_find) │       │
│  │manager)   │ │ websocket)│ │ hook_)     │ │           │       │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘       │
│                         ▲                                        │
│                  Platform Abstraction Layer                      │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Trait: PlatformCapture — start/stop/configure            │   │
│  │  Trait: PlatformHook — inject/detach/on_packet            │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │  #[cfg(windows)] mod wfp  — WFP REDIRECT layer impl  │   │
│  │  #[cfg(windows)] mod hook — SChannel/WinHTTP detour impl │   │
│  │  #[cfg(target_os="macos")] mod nwe  — NetworkExtension  │   │
│  │  #[cfg(target_os="linux")] mod nfq — NFQueue impl       │   │
│  └──────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 核心抽象接口定义

#### 1.2.1 顶层引擎 Trait：`InterceptEngine`

```rust
// crates/engine-core/src/intercept_engine.rs

use async_trait::async_trait;
use tokio::sync::mpsc;

/// 捕获到的 HTTP 消息（请求或响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMessage {
    pub id: u64,
    pub session_id: u64,
    pub direction: MessageDirection,   // Request / Response
    pub protocol: HttpProtocol,        // HTTP1.1 / HTTP2 / WebSocket
    pub scheme: Scheme,                // Http / Https
    pub method: Option<String>,
    pub url: Option<String>,
    pub status_code: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub body_size: usize,
    // --- 元数据 ---
    pub process_name: Option<String>,
    pub process_id: Option<u32>,
    pub process_path: Option<String>,
    pub source_ip: Option<String>,
    pub dest_ip: Option<String>,
    pub source_port: Option<u16>,
    pub dest_port: Option<u16>,
    pub timestamp: u64,                // 微秒精度
    pub duration_us: Option<u64>,      // 请求-响应耗时
    pub cookies: Vec<Cookie>,
    pub raw_tls_info: Option<TlsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    pub version: String,
    pub cipher_suite: String,
    pub server_name: Option<String>,
    pub cert_chain: Vec<Vec<u8>>,      // DER 编码
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub capture_http: bool,
    pub capture_https: bool,
    pub ports: Vec<u16>,               // 空 = 所有端口
    pub process_filters: Vec<String>,  // 空 = 所有进程
    pub max_body_size: usize,          // 超过截断
    pub ca_cert_path: Option<String>,
    pub ca_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub bytes_captured: u64,
    pub tls_handshakes: u64,
    pub hook_injections: u64,
}

#[async_trait]
pub trait InterceptEngine: Send + Sync {
    /// 启动捕获引擎
    async fn start(&mut self, config: CaptureConfig) -> Result<(), EngineError>;

    /// 停止捕获引擎，清理所有资源
    async fn stop(&mut self) -> Result<(), EngineError>;

    /// 动态更新配置（无需重启）
    async fn update_config(&mut self, config: CaptureConfig) -> Result<(), EngineError>;

    /// 获取统计信息
    async fn stats(&self) -> EngineStats;

    /// 安装 CA 根证书到系统信任存储
    async fn install_ca_cert(&self, cert_pem: &str) -> Result<(), EngineError>;

    /// 卸载 CA 根证书
    async fn uninstall_ca_cert(&self) -> Result<(), EngineError>;

    /// 获取当前引擎实现的能力标志
    fn capabilities(&self) -> EngineCapabilities;
}

#[derive(Debug, Clone)]
pub struct EngineCapabilities {
    pub supports_wfp: bool,           // 有 WFP / 等价透明代理
    pub supports_api_hook: bool,      // 有 API Hook
    pub supports_tls_mitm: bool,      // 支持 MITM 解密
    pub supports_http2: bool,
    pub supports_websocket: bool,
    pub process_identification: bool,
}
```

#### 1.2.2 平台捕获 Trait：`PlatformCapture`

```rust
// crates/engine-core/src/platform/capture.rs

#[async_trait]
pub trait PlatformCapture: Send + Sync {
    /// 初始化平台捕获层
    async fn init(&mut self, proxy_addr: SocketAddr) -> Result<(), EngineError>;

    /// 开始重定向流量到 proxy_addr
    async fn start_redirect(&mut self) -> Result<(), EngineError>;

    /// 停止重定向
    async fn stop_redirect(&mut self) -> Result<(), EngineError>;

    /// 查询某连接的原始目标地址（透明代理需要）
    async fn get_original_dst(&self, conn: &TcpStream) -> Result<SocketAddr, EngineError>;

    /// 查询连接归属的进程信息
    async fn resolve_process(&self, local_port: u16) -> Result<ProcessInfo, EngineError>;

    /// 清理资源
    async fn shutdown(&mut self) -> Result<(), EngineError>;
}
```

#### 1.2.3 平台 Hook Trait：`PlatformHook`

```rust
// crates/engine-core/src/platform/hook.rs

pub type HookCallback = Box<dyn Fn(HookPacket) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct HookPacket {
    pub process_id: u32,
    pub process_name: String,
    pub data: Vec<u8>,
    pub direction: Direction,
    pub timestamp: u64,
}

#[async_trait]
pub trait PlatformHook: Send + Sync {
    /// 注入 Hook 到指定进程或全局
    async fn inject(&mut self, targets: &[HookTarget]) -> Result<(), EngineError>;

    /// 卸载 Hook
    async fn detach(&mut self) -> Result<(), EngineError>;

    /// 设置收到数据包时的回调
    fn set_callback(&mut self, cb: HookCallback);

    /// 获取当前 Hook 覆盖的进程列表
    fn covered_processes(&self) -> Vec<ProcessInfo>;
}
```

### 1.3 跨平台扩展策略

| 平台     | 透明代理机制                  | API Hook 机制              | 进程识别            | MITM 证书安装   |
|----------|-------------------------------|----------------------------|---------------------|-----------------|
| Windows  | WFP Redirect Layer (FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4) | Detours / IAT Hook         | `GetExtendedTcpTable` + ETW | CertStore API   |
| macOS    | NetworkExtension (NEPacketTunnelProvider) | `DYLD_INSERT_LIBRARIES` + `fishhook` | `proc_pidinfo` | `security add-trusted-cert` |
| Linux    | NFQueue + iptables/nftables   | `LD_PRELOAD` + `dlsym`     | `/proc/net/tcp`     | NSS / certutil  |
| Android  | VPN Service API               | `LD_PRELOAD` / Frida Gadget | `/proc/net/tcp`    | Trusted CA store|
| iOS      | NetworkExtension              | 不可用（沙盒限制）         | NE 提供             | Configuration Profile |

**关键设计原则：**
- 所有平台差异封装在 `#[cfg(...)]` 条件编译模块内
- `InterceptEngine` trait 的实现类（如 `WindowsEngine`、`MacOSEngine`）在编译时按平台选择
- Tauri 前端的 IPC 接口统一，不感知平台差异
- 共享的协议解析、MITM 逻辑、存储层完全跨平台

---

## 2. 关键技术点解决方案

### 2.1 WFP 连接重定向与 API Hook 协同

#### 双通道互补架构

```
                    ┌─────────────────────────────┐
                    │     User-Mode Application    │
                    │  (browser, curl, app, ...)   │
                    └──────────┬──────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                 │
              ▼                ▼                 ▼
     ┌────────────┐   ┌────────────┐   ┌────────────┐
     │ WinHTTP    │   │ SChannel   │   │ Raw Socket │
     │ API Hook   │   │ API Hook   │   │ connect()  │
     │ (detours)  │   │ (detours)  │   │ (WFP 捕获) │
     └─────┬──────┘   └─────┬──────┘   └─────┬──────┘
           │                │                 │
           └────────────────┼─────────────────┘
                            │
                    ┌───────▼────────┐
                    │  Local Proxy   │  ← 127.0.0.1:40960
                    │  (hyper/h3)    │
                    └───────┬────────┘
                            │
                    ┌───────▼────────┐
                    │  MITM Engine   │
                    │  (rustls/rcgen)│
                    └───────┬────────┘
                            │
                    ┌───────▼────────┐
                    │   Internet     │
                    └────────────────┘
```

#### WFP 覆盖范围 vs API Hook 覆盖范围

| 场景                       | WFP 是否覆盖 | API Hook 是否覆盖 | 说明                               |
|----------------------------|-------------|-------------------|------------------------------------|
| 普通 TCP connect()         | ✅ 覆盖      | ❌ 不覆盖          | WFP 在传输层拦截                    |
| WinHTTP/WinINet API 调用   | ✅ 覆盖      | ✅ 覆盖            | API Hook 可在应用层解析前获取        |
| SChannel TLS 加密前数据     | ❌ 不覆盖    | ✅ 覆盖（可选）    | MITM 解密已提供明文，Hook 非必需      |
| UDP/QUIC (HTTP/3)          | ✅ 覆盖      | ❌ 不覆盖 (目前)   | 通过 WFP ALE 层捕获                 |
| 内核态发起的连接            | ✅ 覆盖      | ❌ 不覆盖          | 只能用 WFP                          |
| 已有 WFP Provider 的应用    | ✅ 覆盖      | ✅ 覆盖            | 我们的 Provider 优先级需设置足够高    |

#### 协同机制

```
1. WFP 作为主通道（重定向层）：
   - 使用系统内置的 FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4 层（Windows 8+）
   - 通过纯用户态 API（fwpuclnt.dll → FwpmFilterAdd）添加重定向过滤规则
   - 在 ALE (Application Layer Enforcement) 层拦截所有 TCP connect() 出站请求
   - 判定是否需要拦截（按端口、进程过滤）
   - 将连接重定向到本地代理端口 127.0.0.1:40960
   - ⚠️ 关键设计决策：REDIRECT 层是系统内置的，无需编写任何内核代码（.sys），无需 EV 签名
   - 不注册自定义 FWPS_CALLOUT / FwpsCalloutRegister（那是内核动作，需要签名驱动）

2. API Hook 作为可选补充通道（应用层）：
   - Hook WinHTTP.dll 的 WinHttpSendRequest / WinHttpReceiveResponse（轻量试点）
   - SChannel Hook（EncryptMessage/DecryptMessage）作为高级可选项，而非必需
   - 在 Hook 函数内读取明文数据后，通过 IPC（Named Pipe 或 mpsc channel）发送给 Engine
   - 不修改原函数的控制流（不篡改/不丢包），只做观察
   - ⚠️ 注意：MITM 解密本身不依赖 API Hook——WFP 重定向确保流量进入代理后，明文由 MITM 解密获得

3. 去重逻辑：
   - 每条流量以 (process_id, local_port, seq) 三元组标识
   - Engine 内维护一个 LRU 缓存（最近 5000 条），对来自 WFP 和 Hook 的数据做去重
   - WFP 负责连接元数据（IP/端口对），Hook 负责明文内容（如有）
```

#### WFP 模块实现框架（纯用户态 REDIRECT，零内核代码）

```rust
// crates/engine-core/src/platform/windows/wfp.rs

use windows::Win32::NetworkManagement::WindowsFilteringPlatform::*;

pub struct WfpRedirector {
    engine_handle: Option<HANDLE>,
    provider_key: GUID,
    sublayer_key: GUID,
    filter_ids: Vec<u64>,
    proxy_port: u16,
}

impl WfpRedirector {
    /// 注册 WFP Provider 和 Sublayer（纯用户态，无需签名）
    pub fn register(&mut self) -> Result<(), WfpError> {
        // FWPM_SESSION — 创建 WFP 动态会话 (进程退出自动清理)
        // FwpmEngineOpen → FwpmTransactionBegin
        // FwpmProviderAdd — 注册 FlowReveal Provider
        // FwpmSubLayerAdd — 注册 Sublayer（设置适当 weight，避免与杀软冲突）
        // FwpmTransactionCommit
    }

    /// 通过 REDIRECT 层添加过滤规则（无需 Callout）
    pub fn add_redirect_filter(&mut self, filter: &CaptureFilter) -> Result<(), WfpError> {
        // FwpmFilterAdd — 在 FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4 添加过滤规则
        // 条件字段：FWPM_CONDITION_IP_PROTOCOL == TCP
        //           FWPM_CONDITION_IP_REMOTE_PORT（按目标端口过滤）
        //           FWPM_CONDITION_ALE_APP_ID（按进程路径过滤）
        //           FWPM_CONDITION_ALE_PACKAGE_ID（排除自身，避免死循环）
        //           FWPM_CONDITION_FLAGS & FWP_CONDITION_FLAG_IS_LOOPBACK（忽略回环）
        // action.type = FWP_ACTION_PERMIT
        // 重定向通过 action 中的 local_address / local_port 指定 127.0.0.1:40960
        // ⚠️ 关键：使用系统内置 REDIRECT 层，不注册任何 FWPS_CALLOUT
    }

    /// 删除所有过滤规则
    pub fn remove_filters(&mut self) -> Result<(), WfpError> {
        for filter_id in &self.filter_ids {
            FwpmFilterDeleteById(self.engine_handle, *filter_id)?;
        }
        self.filter_ids.clear();
        Ok(())
    }

    /// 通过 GetExtendedTcpTable 获取进程归属（兜底方案）
    pub fn resolve_process_id(local_port: u16) -> Option<u32> {
        // 调用 GetExtendedTcpTable，匹配本地端口
        // 返回 owiningPid
        // ⚠️ 注意：该表有秒级延迟，短连接可能已消失，API Hook 的 GetCurrentProcessId() 更精准
    }

    /// 清理资源
    pub fn shutdown(&mut self) -> Result<(), WfpError> {
        self.remove_filters()?;
        // 动态 Session 在进程退出时由系统自动清理
        Ok(())
    }
}
```

### 2.2 HTTPS 解密（MITM）完整流程

#### 整体流程

```
┌────────────────────┐
│  1. 生成 CA 根证书  │
│  (rcgen, 自签名)    │
└────────┬───────────┘
         │
┌────────▼───────────┐
│  2. 安装到系统       │
│  CertAddCTL...      │  → 用户点击 "安装证书" → 优先 CurrentUser\Root（无 UAC）
└────────┬───────────┘     再安装到 LocalMachine\Root（全用户覆盖）
         │                  ⚠️ Firefox 使用 NSS cert9.db，需额外处理
         │
┌────────▼───────────┐
│  3. 客户端发起 TLS   │
│  ClientHello → SNI  │
└────────┬───────────┘
         │
┌────────▼───────────┐
│  4. 动态签发证书      │
│  cache hit? → 复用  │  → LRU Cache <sni, (cert_der, key_der)>
│  cache miss → 签发  │  → rcgen::CertificateParams::new()
└────────┬───────────┘      params.distinguished_name = DN(CN=sni)
         │                  ca_cert.serialize_der_with_signer(&cert)
         │                  cache.insert(sni, cert)
         │
┌────────▼───────────┐
│  5. 完成 TLS 握手     │
│  客户端 ←→ 代理      │  ← Server: 使用签发证书做 Server TLS
│  代理 ←→ 真实服务器    │  ← Client: 代理作为客户端连接真实服务器
└────────┬───────────┘
         │
┌────────▼───────────┐
│  6. 明文传输 & 解析   │
│  解密 HTTP 流量      │
└────────────────────┘
```

#### 根证书生成代码骨架

```rust
// crates/engine-core/src/mitm/ca.rs

use rcgen::{CertificateParams, KeyPair, DistinguishedName, DnType, IsCa, BasicConstraints};
use time::{Duration, OffsetDateTime};

pub struct CertificateAuthority {
    ca_cert: rustls::Certificate,
    ca_key: rustls::PrivateKey,
    cert_cache: LruCache<String, (rustls::Certificate, rustls::PrivateKey)>,
}

impl CertificateAuthority {
    pub fn generate() -> Result<Self, MitmError> {
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "FlowReveal Root CA");
        dn.push(DnType::OrganizationName, "FlowReveal");
        params.distinguished_name = dn;

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::CrlSign,
        ];
        // 有效期 10 年
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(365 * 10);

        let key_pair = KeyPair::generate()?;
        let cert = params.self_signed(&key_pair)?;

        Ok(Self {
            ca_cert: rustls::Certificate(cert.serialize_der()?),
            ca_key: rustls::PrivateKey(key_pair.serialize_der()),
            cert_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        })
    }

    /// 为指定域名签发证书（带缓存）
    pub fn issue_cert(&mut self, sni: &str) -> Result<(rustls::Certificate, rustls::PrivateKey), MitmError> {
        if let Some(cached) = self.cert_cache.get(sni) {
            return Ok(cached.clone());
        }

        let mut params = CertificateParams::new(vec![sni.to_string()]);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, sni);
        params.distinguished_name = dn;
        params.not_before = OffsetDateTime::now_utc();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(365);

        let key_pair = KeyPair::generate()?;
        let ca_params = CertificateParams::from_ca_der(self.ca_cert.0.as_slice())?;
        let ca_key = KeyPair::from_der(self.ca_key.0.as_slice())?;

        let cert = params.signed_by(&key_pair, &ca_params, &ca_key)?;

        let result = (
            rustls::Certificate(cert.serialize_der()?),
            rustls::PrivateKey(key_pair.serialize_der()),
        );
        self.cert_cache.push(sni.to_string(), result.clone());
        Ok(result)
    }
}
```

#### 根证书安装引导（Windows）

```rust
// crates/engine-core/src/platform/windows/cert_store.rs

/// 安装自签名 CA 到 Windows 信任存储
/// 
/// 安装策略（两阶段）：
///   ★ 阶段 1（优先）：安装到 CurrentUser\Root — 无需 UAC 弹窗，无额外管理员提示
///     覆盖范围：Edge、Chrome、IE、以及大多数使用 WinINet/WinHTTP 的应用
///   ★ 阶段 2（补充）：安装到 LocalMachine\Root — 需要管理员权限（已通过 WFP 获取）
///     覆盖范围：系统级服务和所有用户会话
///   ⚠️ 注意：Firefox 使用独立证书商店（NSS cert9.db），需额外处理
///     可使用 certutil 或直接操作 NSS 数据库安装
pub fn install_root_ca(der_bytes: &[u8]) -> Result<(), CertError> {
    unsafe {
        // ★ 阶段 1：优先安装到 CurrentUser\Root（无需额外 UAC 弹窗）
        // 工具本身已需管理员权限运行（WFP），因此此阶段不会额外打断流程
        let user_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            CERT_SYSTEM_STORE_CURRENT_USER,
            windows::core::w!("Root"),
        )?;

        let cert_ctx = CertCreateCertificateContext(
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            der_bytes,
        )?;

        CertAddCertificateContextToStore(
            user_store, cert_ctx, CERT_STORE_ADD_REPLACE_EXISTING, None
        )?;

        // 阶段 2：补充安装到 LocalMachine\Root（覆盖系统级服务和所有用户）
        let lm_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            0,
            0,
            CERT_SYSTEM_STORE_LOCAL_MACHINE,
            windows::core::w!("Root"),
        )?;

        CertAddCertificateContextToStore(
            lm_store, cert_ctx, CERT_STORE_ADD_REPLACE_EXISTING, None
        )?;

        // ⚠️ Firefox 特殊处理：使用独立 NSS 证书数据库
        // 需要额外调用 firefox_cert_install(der_bytes)
        // 或者提示用户手动在 Firefox 中导入
    }
    Ok(())
}
```

#### 证书固定（Certificate Pinning）处理策略

| 方案                   | 效果           | 复杂度 | 适用场景           |
|------------------------|---------------|--------|-------------------|
| 不做对抗              | 应用连接失败    | 低     | 默认行为，记录日志 |
| 应用白名单（绕过 MITM）| 特定应用直连    | 中     | VPN 类/银行类 App  |
| Frida/Objection 注入  | 绕过固定检查    | 高     | 高级分析场景       |
| 修改应用二进制         | 移除固定逻辑    | 极高   | 仅限自研应用       |

**推荐默认策略**：在 UI 中显示连接失败的 Host 列表，让用户自行选择是否加入 "Bypass MITM" 白名单。

### 2.3 进程识别

#### 三条途径互补

```
途径1：API Hook 直接获取（★ 主方案，最精准）
  ─ 在 SChannel/WinHTTP Hook 回调中，直接携带 GetCurrentProcessId()
  ─ 无需额外查询，天然精准，零延迟
  ─ 进程名称通过 OpenProcess + QueryFullProcessImageNameW 补充

途径2：GetExtendedTcpTable (IP Helper API) — 兜底方案
  ─ 每 1 秒轮询一次 MIB_TCPTABLE_OWNER_PID
  ─ 建立 (local_addr, local_port) → pid 的缓存映射
  ─ 在 transparent proxy 收到连接时，通过 getsockname() 获取本地端口查询
  ─ ⚠️ 局限性：秒级延迟，高并发/短连接场景下表项可能已消失（连接关闭→表项消失→无法识别）
  ─ ⚠️ 建议在阶段 2 实施时重点测试进程识别命中率，若低于 95% 需升级方案

途径3：ETW Microsoft-Windows-TCPIP 事件（高命中率兜底，无需驱动）
  ─ 通过 TraceLogging 订阅 Microsoft-Windows-TCPIP 提供者的实时事件
  ─ 可获取 TCP 连接建立/关闭时的 (src_ip, src_port, dst_ip, dst_port, pid) 对应关系
  ─ 延迟极低（内核直接派发），远优于轮询 GetExtendedTcpTable
  ─ 无需额外驱动，纯用户态 ETW 订阅即可
  ─ 建议在进程识别命中率 < 95% 时引入 ETW 通道
```

```rust
// crates/engine-core/src/platform/windows/process_resolver.rs

use std::collections::HashMap;
use tokio::time::{interval, Duration};

pub struct ProcessResolver {
    port_pid_cache: HashMap<u16, u32>,
    pid_info_cache: LruCache<u32, ProcessInfo>,
    hit_rate: f64,          // 进程识别命中率监控
    total_queries: u64,
    misses: u64,
}

impl ProcessResolver {
    /// 后台任务：每 1 秒刷新 TCP 连接表
    /// ⚠️ 局限性：秒级延迟，短连接可能已消失导致命中率下降
    pub async fn refresh_loop(&mut self) {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            self.refresh_tcp_table();
        }
    }

    fn refresh_tcp_table(&mut self) {
        // GetExtendedTcpTable 获取 MIB_TCPTABLE_OWNER_PID
        // 遍历所有连接，更新 port_pid_cache
    }

    pub fn resolve_by_port(&self, local_port: u16) -> Option<ProcessInfo> {
        let pid = self.port_pid_cache.get(&local_port)?;
        self.resolve_by_pid(*pid)
    }

    pub fn resolve_by_pid(&self, pid: u32) -> Option<ProcessInfo> {
        // OpenProcess + QueryFullProcessImageNameW
        // 返回 (name, path, pid)
    }

    /// 报告命中率（监控指标）
    pub fn hit_rate(&self) -> f64 {
        if self.total_queries == 0 { return 1.0; }
        self.hit_rate
    }
}

/// ETW 备选方案（当轮询命中率 < 95% 时启用）
/// 订阅 Microsoft-Windows-TCPIP 提供者，实时获取 (port → PID) 映射
/// 延迟极低（内核直接派发），无需驱动
pub struct EtwProcessTracker { /* ... */ }
```

### 2.4 HTTP/2 及 WebSocket 明文化解析

#### HTTP/2 解析策略

```
MITM 代理充当 HTTP/2 协议转换器：

  客户端                        代理                       服务器
    │                            │                          │
    │──── TLS 连接 ──────────────▶│                          │
    │    (h2 ALPN)               │                          │
    │                            │──── TLS 连接 ───────────▶│
    │                            │    (h2 ALPN)             │
    │                            │                          │
    │──── HTTP/2 HEADERS ───────▶│                          │
    │    (HPACK 压缩)             │  ┌─────────────────┐    │
    │                            │  │ 解压 → 存储 →     │    │
    │                            │  │ 记录到 UI         │    │
    │                            │  └─────────────────┘    │
    │                            │──── HTTP/2 HEADERS ────▶│
    │                            │    (重新 HPACK 编码)     │
```

使用 `h2` crate 做帧级别的解析，`hpack` 做头部压缩/解压：

```rust
// crates/engine-core/src/protocol/http2.rs

use h2::server;
use h2::client;
use tokio::net::TcpStream;

pub async fn proxy_http2(
    client_conn: TcpStream,
    server_conn: TcpStream,
    interceptor: &Interceptor,
) -> Result<(), H2Error> {
    // 1. 服务端：接受客户端 HTTP/2 连接
    let mut server = server::handshake(client_conn).await?;
    // 2. 客户端：连接上游
    let (mut client, client_conn) = client::handshake(server_conn).await?;

    // 3. 双向桥接
    while let Some(result) = server.accept().await {
        let (req, respond) = result?;
        // 记录请求到 interceptor
        interceptor.on_h2_request(&req).await;

        // 转发到上游
        let resp = client.send_request(req).await?;
        let (parts, body) = resp.into_parts();

        // 记录响应
        interceptor.on_h2_response(&parts).await;

        // 返回给客户端
        respond.send_response(resp).await?;
    }
    Ok(())
}
```

#### WebSocket 解析策略

```
使用 tungstenite / tokio-tungstenite：

  - 客户端 → 代理 WebSocket 升级握手（记录 Upgrade 请求）
  - 代理 → 上游 WebSocket 升级握手
  - 握手成功后，双向转发每条 WebSocket Frame
  - 每条 Frame 以 Message 为单位记录到 UI：
      ┌──────────────────────────────────┐
      │ [#42] WS → ws://echo.example.com │
      │ Opcode: Text                     │
      │ Payload: {"hello":"world"}       │
      │ Timestamp: 14:32:01.234          │
      │ Direction: Outbound              │
      └──────────────────────────────────┘
```

```rust
// crates/engine-core/src/protocol/websocket.rs

use tokio_tungstenite::tungstenite::Message;

pub struct WsInterceptor {
    session_id: u64,
    tx: mpsc::Sender<HttpMessage>,
}

impl WsInterceptor {
    pub async fn relay_and_record(
        &self,
        client_ws: &mut WebSocketStream<TcpStream>,
        server_ws: &mut WebSocketStream<TcpStream>,
    ) {
        loop {
            tokio::select! {
                msg = client_ws.next() => {
                    if let Some(Ok(msg)) = msg {
                        self.record_outbound(&msg).await;
                        server_ws.send(msg).await;
                    }
                }
                msg = server_ws.next() => {
                    if let Some(Ok(msg)) = msg {
                        self.record_inbound(&msg).await;
                        client_ws.send(msg).await;
                    }
                }
            }
        }
    }
}
```

---

## 3. 详细开发路线图

### 阶段 1：Tauri + React 骨架 + 基本正向代理

**目标**：搭建可运行的桌面应用框架，实现最简单的 HTTP 正向代理，在 UI 中实时展示请求列表。

**文件结构（新增/修改）**：

```
FlowReveal/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs                    # Tauri 入口
│   │   ├── lib.rs                     # 模块导出
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   └── proxy_commands.rs      # start/stop/get_requests IPC
│   │   ├── proxy/
│   │   │   ├── mod.rs
│   │   │   └── forward_proxy.rs       # 简单 HTTP 正向代理
│   │   └── state.rs                   # 全局状态 AppState
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── TrafficList.tsx            # 请求列表
│   │   ├── RequestDetail.tsx          # 请求详情面板
│   │   ├── StatusBar.tsx              # 底部状态栏
│   │   └── Toolbar.tsx                # 顶部工具栏
│   ├── hooks/
│   │   └── useTraffic.ts              # 接收后端推送的 Hook
│   ├── types/
│   │   └── index.ts                   # TypeScript 类型定义
│   └── styles/
│       └── global.css
├── package.json
├── tsconfig.json
└── vite.config.ts
```

**`forward_proxy.rs` 核心骨架**：

```rust
// src-tauri/src/proxy/forward_proxy.rs

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, Method};
use hyper::upgrade::Upgraded;
use tokio::io::copy_bidirectional;
use std::net::SocketAddr;
use tokio::sync::broadcast;

pub struct ForwardProxy {
    addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    event_tx: broadcast::Sender<HttpMessage>,
}

impl ForwardProxy {
    pub fn new(port: u16) -> Self {
        let (event_tx, _) = broadcast::channel::<HttpMessage>(1024);
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            shutdown_tx: None,
            event_tx,
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let event_tx = self.event_tx.clone();

        let make_svc = make_service_fn(move |_conn| {
            let event_tx = event_tx.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    handle_request(req, event_tx.clone())
                }))
            }
        });

        let server = Server::bind(&self.addr)
            .serve(make_svc)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });

        server.await?;
        Ok(())
    }
}

async fn handle_request(
    req: Request<Body>,
    event_tx: broadcast::Sender<HttpMessage>,
) -> Result<Response<Body>, hyper::Error> {
    if req.method() == Method::CONNECT {
        // HTTPS 隧道（阶段 1 不处理，转阶段 3）
        Ok(Response::new(Body::from("CONNECT not supported yet")))
    } else {
        // 转发 HTTP 请求
        let client = hyper::Client::new();
        let resp = client.request(req).await?;

        // 记录到事件总线
        let msg = HttpMessage { /* ... from req & resp ... */ };
        let _ = event_tx.send(msg);

        Ok(resp)
    }
}
```

**`proxy_commands.rs` — Tauri IPC 对接**：

```rust
// src-tauri/src/commands/proxy_commands.rs

use tauri::{command, State, Manager};
use crate::state::AppState;

#[command]
pub async fn start_proxy(state: State<'_, AppState>, port: u16) -> Result<(), String> {
    let mut proxy = state.proxy.lock().await;
    *proxy = Some(ForwardProxy::new(port));
    // 启动后台任务
    let mut p = proxy.take().unwrap();
    tokio::spawn(async move {
        p.start().await;
    });
    Ok(())
}

#[command]
pub async fn stop_proxy(state: State<'_, AppState>) -> Result<(), String> {
    // 发送 shutdown 信号
    Ok(())
}

#[command]
pub async fn get_requests(state: State<'_, AppState>) -> Result<Vec<HttpMessage>, String> {
    Ok(state.requests.lock().await.clone())
}
```

**前端 IPC 通信方式**：

```typescript
// src/hooks/useTraffic.ts

import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

export function useTraffic() {
  const [requests, setRequests] = useState<HttpMessage[]>([]);

  useEffect(() => {
    const unlisten = listen<HttpMessage>('traffic://request', (event) => {
      setRequests(prev => [...prev, event.payload]);
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  return { requests };
}
```

**阶段 1 完成标志**：
- ✅ Tauri + React 应用能运行
- ✅ 浏览器配置 `127.0.0.1:40960` 作为代理后，UI 中能实时看到 HTTP 请求列表
- ✅ 请求列表显示 URL、Method、Status Code、耗时
- ✅ 点击某条请求可展开 Headers/Body 详情

---

### 阶段 2：WFP 透明代理

**目标**：通过 WFP 将系统出站流量重定向到本地代理，无需手动设置系统代理。

**文件结构（新增）**：

```
src-tauri/src/
├── platform/
│   ├── mod.rs
│   └── windows/
│       ├── mod.rs
│       ├── wfp.rs                # WFP 注册/过滤/REDIRECT 重定向（纯用户态）
│       ├── process_resolver.rs   # 进程解析
│       └── ffi_helpers.rs        # Win32 FFI 封装
├── proxy/
│   ├── mod.rs
│   ├── forward_proxy.rs
│   └── transparent_proxy.rs     # 透明代理（REDIRECT 模式）
└── state.rs
```

**`wfp.rs` 核心骨架**：

```rust
// src-tauri/src/platform/windows/wfp.rs

use windows::Win32::NetworkManagement::WindowsFilteringPlatform::*;

/// 使用系统内置 FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4 层
/// 纯用户态 API，不编写任何内核代码（.sys），无需 EV 签名
pub fn add_redirect_rule(
    engine: HANDLE,
    proxy_port: u16,
    target_ports: &[u16],
) -> Result<u64, WfpError> {
    // 1. 构建过滤条件
    //    - FWPM_CONDITION_IP_PROTOCOL == TCP
    //    - FWPM_CONDITION_IP_REMOTE_PORT 在 target_ports 范围内
    //    - FWPM_CONDITION_ALE_PACKAGE_ID 排除自身进程 SID（避免死循环）
    //    - FWPM_CONDITION_FLAGS & FWP_CONDITION_FLAG_IS_LOOPBACK（忽略回环流量）
    //
    // 2. 构建重定向 Action
    //    - action.type = FWP_ACTION_PERMIT
    //    - action 包含 local_address = 127.0.0.1, local_port = proxy_port
    //
    // 3. 调用 FwpmFilterAdd 添加到 REDIRECT 层
    //    - layerKey = FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4
    //    - subLayerKey = 我们的 Sublayer
    //
    // 返回 filter_id 用于后续删除
}
```

**透明代理与正向代理的区别**：

```rust
// transparent_proxy.rs — 关键处理
//
// 正向代理收到：GET http://example.com/path HTTP/1.1
// 透明代理收到：GET /path HTTP/1.1  (缺少 scheme + host)
//
// 需要通过 getsockname / SO_ORIGINAL_DST 获取原始目标地址

async fn handle_transparent(
    req: Request<Body>,
    original_dst: SocketAddr,
) -> Result<Response<Body>> {
    // 从 getsockname / SO_ORIGINAL_DST 获取的 original_dst
    let host = original_dst.to_string();

    // 重建完整 URI
    let uri = format!("http://{}{}", host, req.uri().path_and_query()
        .map(|p| p.as_str()).unwrap_or("/"));
    let (mut parts, body) = req.into_parts();
    parts.uri = uri.parse()?;
    parts.headers.insert("Host", host.parse()?);

    let req = Request::from_parts(parts, body);
    // 正常转发...
}
```

**阶段 2 完成标志**：
- ✅ 开启捕获后，浏览器无需配置代理即可被捕获 HTTP 流量
- ✅ 流量列表正确显示进程名/ID
- ✅ 透明代理正确处理 HTTP 请求
- ✅ WFP 注册/清理在应用关闭时自动完成（动态 Session）

---

### 阶段 3：HTTPS MITM 解密引擎

**目标**：实现 CA 签发、TLS 中间人解密、HTTPS 明文展示。

**文件结构（新增）**：

```
src-tauri/src/
├── mitm/
│   ├── mod.rs
│   ├── ca.rs                 # CA 生成、缓存
│   ├── tls_interceptor.rs    # TLS 拦截层
│   └── cert_store.rs         # 证书存储 (PEM/DER)
├── platform/windows/
│   ├── cert_store_win.rs     # Windows 证书安装
│   └── cert_install_gui.rs   # 根证书安装引导 UI
```

**`tls_interceptor.rs` 核心骨架**：

```rust
// src-tauri/src/mitm/tls_interceptor.rs

use rustls::{ServerConfig, ClientConfig, ServerConnection, ClientConnection};
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::TlsConnector;
use std::sync::Arc;

pub struct TlsInterceptor {
    ca: CertificateAuthority,
    server_config_cache: LruCache<String, Arc<ServerConfig>>,
}

impl TlsInterceptor {
    /// 处理 CONNECT 请求，建立 MITM 隧道
    pub async fn handle_connect(
        &mut self,
        client_conn: TcpStream,
        sni: &str,
        original_dst: SocketAddr,
    ) -> Result<(), MitmError> {
        // 1. 为 sni 签发/获取证书
        let (cert, key) = self.ca.issue_cert(sni)?;

        // 2. 构建 TLS Server Config（面向客户端）
        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        // 3. 与客户端完成 TLS 握手
        let client_tls = acceptor.accept(client_conn).await?;

        // 4. 连接真实服务器
        let server_conn = TcpStream::connect(original_dst).await?;

        // 5. 与服务器完成 TLS 握手
        let mut root_store = rustls::RootCertStore::empty();
        // 加载系统根证书
        root_store.add_parsable_certificates(&rustls_native_certs::load_native_certs()?);

        let client_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(client_config));

        let server_tls = connector.connect(
            rustls::ServerName::try_from(sni)?,
            server_conn,
        ).await?;

        // 6. 双向转发明文
        Self::relay_plaintext(client_tls, server_tls, sni).await?;

        Ok(())
    }

    async fn relay_plaintext(
        client: tokio_rustls::server::TlsStream<TcpStream>,
        server: tokio_rustls::client::TlsStream<TcpStream>,
        sni: &str,
    ) -> Result<()> {
        let (mut client_r, mut client_w) = tokio::io::split(client);
        let (mut server_r, mut server_w) = tokio::io::split(server);

        // 使用 hyper 的 HTTP/1.1 连接器解析双向流量
        let client_conn = hyper::server::conn::Http::new();
        let server_conn = hyper::client::conn::Http1::new();

        // 建立双向 HTTP 管道
        // ... 详细的请求/响应转发代码
    }
}
```

**阶段 3 完成标志**：
- ✅ 用户点击 "安装根证书" 后，CA 自动安装到系统信任区
- ✅ 访问 `https://example.com` 在 UI 中能看到明文 Headers/Body
- ✅ 证书缓存生效（同一 SNI 不重复签发）
- ✅ 遇到证书固定时，连接失败被优雅记录而非崩溃

---

### 阶段 4：API Hook 协同（可选增强）

**目标**：引入 WinHTTP Hook 作为轻量试点，捕获应用层明文数据，与 WFP 互补。SChannel Hook 作为高级可选项。

**核心原则**：
- API Hook 在解密链路上是**锦上添花，非必需**——WFP 重定向确保流量进入代理后，明文由 MITM 解密获得
- 优先 Hook WinHTTP.dll / WinINet.dll（应用层高层 API，Hook 风险低，不受 PatchGuard 影响）
- SChannel Hook（EncryptMessage/DecryptMessage）涉及系统 DLL 内联 Hook，64 位系统受 PatchGuard 和杀软防护，作为高级可选功能
- 阶段 4 先实现 WinHTTP Hook 作为试点，视稳定性再决定是否深入 SChannel

**文件结构（新增）**：

```
src-tauri/src/
├── platform/windows/
│   ├── hook/
│   │   ├── mod.rs
│   │   ├── hook_engine.rs         # Hook 引擎总控
│   │   ├── schannel_hook.rs       # SChannel Hook
│   │   ├── winhttp_hook.rs        # WinHTTP Hook
│   │   ├── detour_trampoline.rs   # Detours 跳板函数
│   │   ├── injector.rs            # DLL 注入器
│   │   └── ipc_pipe.rs            # Hook → Engine 的 Named Pipe IPC
│   └── wfp.rs
└── hook_dll/                       # 独立编译的注入 DLL (Rust cdylib)
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── schannel.rs
        └── winhttp.rs
```

**Hook 架构设计**：

```
┌──────────────────────────────────────────────────────────┐
│                   FlowReveal Engine (Rust)                │
│  ┌─────────────┐                                         │
│  │ hook_engine │ ← 管理 Hook 生命周期                      │
│  │             │ ← spawn_hook_injector()                   │
│  └──────┬──────┘                                         │
│         │ CreateRemoteThread / SetWindowsHookEx            │
│         │                                                  │
│         ▼                                                  │
│  ┌─────────────┐     Named Pipe IPC      ┌─────────────┐ │
│  │ Named Pipe  │◄───────────────────────►│ flowreveal_ │ │
│  │ Server      │   hook_event channels   │ hook.dll    │ │
│  └─────────────┘                         └──────┬──────┘ │
│                                                  │         │
│                                    Detours Hook   │         │
│                                                  ▼         │
│                         ┌────────────────────────────────┐│
│                         │  SChannel.dll / WinHTTP.dll     ││
│                         │  EncryptMessage → hook_fn       ││
│                         │  DecryptMessage → hook_fn       ││
│                         │  WinHttpSendRequest → hook_fn   ││
│                         └────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

**`hook_dll/src/lib.rs` 核心骨架**：

```rust
// hook_dll/src/lib.rs — 编译为 cdylib，注入目标进程

use std::ffi::c_void;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

#[no_mangle]
pub extern "system" fn DllMain(_hinst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    match reason {
        1 => { // DLL_PROCESS_ATTACH
            std::thread::spawn(|| {
                hook_loop();
            });
        }
        _ => {}
    }
    1
}

fn hook_loop() {
    // 1. 连接到 Engine 的 Named Pipe
    let pipe = connect_to_pipe(r"\\.\pipe\flowreveal_hook");
    // 2. 安装 Detours
    install_schannel_hooks(pipe.clone());
    install_winhttp_hooks(pipe.clone());
    // 3. 保持 DLL 常驻
    std::thread::park();
}

fn install_schannel_hooks(pipe: NamedPipe) {
    unsafe {
        // detour SChannel!EncryptMessage
        // 在 hook_fn 中：
        //   1. 调用原始 EncryptMessage（保持功能正常）
        //   2. 如果 pInputBuffers 中有明文 → 通过 pipe 发送给 Engine
    }
}
```

**`hook_engine.rs` — Engine 侧注入管理**：

```rust
// src-tauri/src/platform/windows/hook/hook_engine.rs

pub struct HookEngine {
    target_pids: Vec<u32>,
    pipe_server: NamedPipeServer,
}

impl HookEngine {
    /// 向指定进程注入 hook DLL
    pub fn inject_into_process(&self, pid: u32) -> Result<(), HookError> {
        unsafe {
            let process = OpenProcess(
                PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION |
                PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_VM_READ,
                false, pid,
            )?;

            let dll_path = Self::get_dll_path();
            let dll_path_wide: Vec<u16> = dll_path.encode_utf16().chain(Some(0)).collect();

            // 在目标进程分配内存
            let alloc_addr = VirtualAllocEx(
                process, None,
                dll_path_wide.len() * 2,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );

            // 写入 DLL 路径
            WriteProcessMemory(process, alloc_addr,
                dll_path_wide.as_ptr() as *const _,
                dll_path_wide.len() * 2, None);

            // 通过 CreateRemoteThread → LoadLibraryW 执行注入
            let load_lib = GetProcAddress(GetModuleHandleW(w!("kernel32.dll"))?, "LoadLibraryW");
            let thread = CreateRemoteThread(process, None, 0,
                Some(std::mem::transmute(load_lib)),
                alloc_addr, 0, None);

            WaitForSingleObject(thread, 5000);
            Ok(())
        }
    }

    /// 全局钩子方式（备选：使用 SetWindowsHookEx 实现更广覆盖）
    pub fn install_global_hook(&self) -> Result<(), HookError> {
        // SetWindowsHookEx(WH_GETMESSAGE, hook_proc, hmod, 0)
        // 注入到所有 GUI 线程的消息队列中
    }

    /// 接收来自各 Hook DLL 的事件
    pub async fn event_loop(&mut self) {
        loop {
            let event = self.pipe_server.read_event().await;
            // 解析 Hook 事件，去重后推送到 Engine 的 event_tx
        }
    }
}
```

**Detours 跳板函数设计**：

```rust
// hook_dll/src/schannel.rs — SChannel Hook 的 Detours 跳板

use windows::Win32::Security::Cryptography::Ssl::*;

// 原始函数指针（跳板）
static mut REAL_ENCRYPT_MESSAGE: Option<
    unsafe extern "system" fn(
        phContext: *mut c_void,
        fQualityOfProtection: u32,
        pMessage: *mut SecBufferDesc,
        dwMessageSeq: u32,
    ) -> u32
> = None;

// 我们的 Hook 函数
unsafe extern "system" fn encrypt_message_hook(
    phContext: *mut c_void,
    fQualityOfProtection: u32,
    pMessage: *mut SecBufferDesc,
    dwMessageSeq: u32,
) -> u32 {
    // 1. 先读取明文数据（在加密之前）
    if let Some(msg) = extract_plaintext(pMessage) {
        send_to_engine_via_pipe(&msg);
    }

    // 2. 调用原始函数（保持原逻辑不变）
    REAL_ENCRYPT_MESSAGE.unwrap()(phContext, fQualityOfProtection, pMessage, dwMessageSeq)
}

pub fn install_schannel_hooks() {
    unsafe {
        let module = GetModuleHandleW(w!("schannel.dll")).unwrap();
        let encrypt_addr = GetProcAddress(module, "EncryptMessage");

        // 使用 detour 库安装 Hook
        DetourTransactionBegin();
        DetourUpdateThread(GetCurrentThread());
        DetourAttach(&mut REAL_ENCRYPT_MESSAGE as *mut _ as _, encrypt_message_hook as *mut _);
        DetourTransactionCommit();
    }
}
```

**阶段 4 完成标志**：
- ✅ WinHTTP Hook DLL 能成功注入到目标进程
- ✅ WinHttpSendRequest / WinHttpReceiveResponse 被拦截，请求信息流入 Engine
- ✅ Hook 数据与 WFP 数据去重正确
- ✅ 进程密集注入不造成系统不稳定
- ✅ （可选）SChannel Hook 试点验证，仅在稳定性验证通过后启用

---

### 阶段 5：高级功能完善

**目标**：HTTP/2 解析、WebSocket 支持、请求过滤/搜索/重放、导出。

**文件结构（新增）**：

```
src-tauri/src/
├── protocol/
│   ├── mod.rs
│   ├── http1.rs           # HTTP/1.1 解析器
│   ├── http2.rs           # HTTP/2 代理与解析
│   ├── websocket.rs       # WebSocket 帧记录
│   └── cookie_jar.rs      # Cookie 聚合分析
├── storage/
│   ├── mod.rs
│   ├── session_store.rs   # 会话持久化（SQLite）
│   ├── har_export.rs      # HAR 格式导出
│   └── json_export.rs     # JSON/CSV 导出
├── replay/
│   ├── mod.rs
│   └── request_replay.rs  # 请求重放引擎
├── filter/
│   ├── mod.rs
│   ├── filter_engine.rs   # 过滤规则引擎
│   └── search_index.rs    # 全文搜索索引
src/
├── components/
│   ├── FilterBar.tsx       # 过滤栏
│   ├── SearchPanel.tsx     # 搜索面板
│   ├── ReplayDialog.tsx    # 重放对话框
│   ├── ExportDialog.tsx    # 导出对话框
│   └── WebSocketView.tsx   # WebSocket 消息视图
└── hooks/
    ├── useFilter.ts
    ├── useSearch.ts
    └── useExport.ts
```

**请求过滤 DSL**：

```typescript
// 过滤语法示例（编译为 AST 后在 Engine 端执行）
// method:GET host:api.example.com status:200..299
// proc:chrome body:token min-size:1KB max-size:1MB

interface FilterRule {
  field: 'method' | 'host' | 'path' | 'status' | 'proc' | 'body' | 'content-type' | 'duration' | 'size';
  operator: 'eq' | 'contains' | 'regex' | 'gt' | 'lt' | 'range';
  value: string | number | [number, number];
}
```

**HAR 导出骨架**：

```rust
// src-tauri/src/storage/har_export.rs

use serde_json::json;

pub fn export_har(sessions: &[HttpSession]) -> String {
    let entries: Vec<serde_json::Value> = sessions.iter().map(|s| {
        json!({
            "startedDateTime": iso_timestamp(s.request.timestamp),
            "time": s.request.duration_us.unwrap_or(0) / 1000,
            "request": {
                "method": s.request.method,
                "url": s.request.url,
                "httpVersion": "HTTP/1.1",
                "headers": headers_to_har(&s.request.headers),
                "cookies": cookies_to_har(&s.request.cookies),
                "headersSize": -1,
                "bodySize": s.request.body_size,
                "postData": s.request.body.as_ref().map(|b| json!({
                    "mimeType": s.request.content_type(),
                    "text": String::from_utf8_lossy(b),
                })),
            },
            "response": {
                "status": s.response.as_ref().map(|r| r.status_code),
                "statusText": s.response.as_ref().map(|r| status_text(r.status_code.unwrap_or(0))),
                "httpVersion": "HTTP/1.1",
                "headers": s.response.as_ref().map(|r| headers_to_har(&r.headers)),
                "cookies": s.response.as_ref().map(|r| cookies_to_har(&r.cookies)),
                "content": s.response.as_ref().and_then(|r| r.body.as_ref()).map(|b| json!({
                    "size": b.len(),
                    "text": String::from_utf8_lossy(b),
                })),
                "redirectURL": "",
                "headersSize": -1,
                "bodySize": s.response.as_ref().map(|r| r.body_size).unwrap_or(0),
            },
            "cache": {},
            "timings": {
                "send": 0,
                "wait": s.request.duration_us.unwrap_or(0) / 1000,
                "receive": 0,
            },
            "serverIPAddress": s.request.dest_ip,
            "connection": s.request.dest_port.map(|p| p.to_string()).unwrap_or_default(),
        })
    }).collect();

    let har = json!({
        "log": {
            "version": "1.2",
            "creator": { "name": "FlowReveal", "version": env!("CARGO_PKG_VERSION") },
            "entries": entries,
        }
    });

    serde_json::to_string_pretty(&har).unwrap()
}
```

**阶段 5 完成标志**：
- ✅ HTTP/2 连接被正确代理，多路复用流被解析为独立条目
- ✅ WebSocket 消息逐帧记录
- ✅ 支持丰富的过滤条件（method、URL、状态码、进程名、内容搜索）
- ✅ 支持 HAR 和 JSON 格式导出
- ✅ 请求重放功能正常

---

### 阶段 6：兼容性测试、打包、性能优化与跨平台接口

**目标**：稳定化、免安装打包、性能调优、跨平台抽象文档。

**任务清单**：

#### 6.1 免安装打包

```toml
# src-tauri/Cargo.toml — bundler 配置
[package]
name = "flowreveal"
version = "1.0.0"
build.target = "x86_64-pc-windows-msvc"

[tauri.bundle]
active = true
targets = ["nsis", "msi"]
icon = ["icons/icon.ico"]

[tauri.bundle.windows]
wix = { language = "zh-CN" }
nsis = {
    installMode = "currentUser",
    languages = ["SimpChinese"]
}

# 免安装版本：直接分发 tauri build 产生的 .exe + resources
# 所有依赖静态链接，单个文件夹内包含 flowreveal.exe 和 WebView2 resources
```

推荐分发方式：
1. **便携版**：`tauri build --bundles nsis` 生成单文件安装包
2. **绿色版**：直接打 zip，解压即用。需要 `WebView2` 运行时（Win10 1809+ 系统已内置）
3. **签名**：获取 EV Code Signing Certificate 对 exe 签名，避免 SmartScreen 拦截

#### 6.2 性能优化

| 优化项              | 方案                                               | 预期收益     |
|---------------------|---------------------------------------------------|-------------|
| Body 截断           | 超过 `max_body_size` 截断并标记 `truncated: true`    | 内存降 70%   |
| LRU 淘汰            | 请求列表保留最近 10000 条，旧数据写入 SQLite          | UI 不卡顿    |
| 连接池复用          | hyper 的 Client 配置 `pool_max_idle_per_host(5)`     | 减少握手耗时 |
| 证书缓存            | 使用 LRU + 预签发（常用 SNI 提前签发）               | TLS 握手提速|
| Async I/O           | 全程 Tokio runtime，零阻塞等待                       | 吞吐最大化  |
| IPC 批处理          | 前端批量拉取请求列表（每次 50 条），非逐条推送         | UI 帧率 > 30 |

#### 6.3 兼容性测试矩阵

| 测试项             | Windows 10 21H2 | Windows 11 23H2 | Windows Server 2022 |
|--------------------|:---:|:---:|:---:|
| WFP 透明代理        | ✅  | ✅  | ✅  |
| HTTPS MITM          | ✅  | ✅  | ✅  |
| SChannel Hook       | ✅  | ✅  | ✅  |
| WinHTTP Hook        | ✅  | ✅  | ✅  |
| Edge/Chrome/Firefox | ✅  | ✅  | ✅  |
| .NET HttpClient     | ✅  | ✅  | ✅  |
| Python requests     | ✅  | ✅  | ✅  |
| Go net/http         | ✅  | ✅  | ✅  |
| curl.exe            | ✅  | ✅  | ✅  |
| Electron App        | ✅  | ✅  | ✅  |

#### 6.4 跨平台接口文档

为 macOS/Linux 预留的 PlatformCapture / PlatformHook 实现骨架：

```rust
// crates/engine-core/src/platform/linux/nfqueue.rs

#[cfg(target_os = "linux")]
pub struct NfQueueCapture {
    queue_num: u16,
    nfq_handle: Option<NfqHandle>,
    proxy_port: u16,
}

#[cfg(target_os = "linux")]
impl PlatformCapture for NfQueueCapture {
    async fn init(&mut self, proxy_addr: SocketAddr) -> Result<(), EngineError> {
        // 1. 使用 nftnl / nfqueue crate 打开 NFQueue
        // 2. 设置 nftables 规则: iptables -t nat -A OUTPUT -p tcp -j NFQUEUE --queue-num 0
        // 3. 在回调中修改目标地址为 proxy_addr
    }
    async fn get_original_dst(&self, conn: &TcpStream) -> Result<SocketAddr, EngineError> {
        // SO_ORIGINAL_DST sockopt
    }
}
```

```rust
// crates/engine-core/src/platform/macos/nwe.rs

#[cfg(target_os = "macos")]
pub struct NetworkExtensionCapture {
    packet_tunnel: Option<PacketTunnelProvider>,
}

#[cfg(target_os = "macos")]
impl PlatformCapture for NetworkExtensionCapture {
    async fn init(&mut self, proxy_addr: SocketAddr) -> Result<(), EngineError> {
        // 1. 加载 NetworkExtension System Extension
        // 2. 启动 NEPacketTunnelProvider
        // 3. 在 handleAppMessage 中转发到本地代理
    }
}
```

**阶段 6 完成标志**：
- ✅ 打包为单个文件夹，复制到新机器启动即用
- ✅ 在 10 台以上不同配置的 Windows 10/11 机器测试通过
- ✅ 内存占用 < 200MB（捕获 10000 条请求基准）
- ✅ CPU 占用 < 5%（待机状态）
- ✅ 跨平台扩展文档完整

---

## 4. 代码组织与模块依赖

### 4.1 Git 仓库目录结构

```
FlowReveal/                           # Monorepo 根
├── .github/
│   └── workflows/
│       ├── ci.yml                    # Rust 编译 + lint + test
│       ├── release.yml               # Tauri 打包 + 签名
│       └── codeql.yml                # CodeQL 安全扫描
├── crates/                           # Rust Workspace
│   ├── engine-core/                  # 核心引擎（跨平台）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── intercept_engine.rs   # InterceptEngine trait
│   │       ├── http_message.rs       # HttpMessage 结构体
│   │       ├── capture_config.rs     # CaptureConfig
│   │       ├── engine_error.rs       # EngineError 枚举
│   │       ├── mitm/
│   │       │   ├── mod.rs
│   │       │   ├── ca.rs             # CA 生成 + 签名（rcgen）
│   │       │   └── cert_cache.rs     # 证书缓存
│   │       ├── protocol/
│   │       │   ├── mod.rs
│   │       │   ├── http1.rs          # HTTP/1.1 请求/响应解析
│   │       │   ├── http2.rs          # HTTP/2 帧解析
│   │       │   ├── websocket.rs      # WS 帧记录
│   │       │   └── cookie.rs         # Cookie 提取与 Jar
│   │       ├── platform/             # 平台抽象层
│   │       │   ├── mod.rs
│   │       │   ├── capture.rs        # PlatformCapture trait
│   │       │   ├── hook.rs           # PlatformHook trait
│   │       │   ├── windows/
│   │       │   │   ├── mod.rs
│   │       │   │       ├── wfp.rs        # WFP REDIRECT 重定向（纯用户态，零内核）
│   │       │   │       ├── process_resolver.rs
│   │       │   │       ├── etw_tracker.rs  # ETW TCP/IP 事件订阅（进程识别兜底）
│   │       │   │   ├── cert_store.rs
│   │       │   │   └── shared_memory.rs # 进程间共享内存
│   │       │   ├── linux/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── nfqueue.rs
│   │       │   │   └── ld_preload.rs
│   │       │   └── macos/
│   │       │       ├── mod.rs
│   │       │       └── nwe.rs
│   │       ├── storage/
│   │       │   ├── mod.rs
│   │       │   ├── session.rs        # Session 数据结构
│   │       │   ├── sqlite_store.rs   # SQLite 持久化
│   │       │   ├── har_export.rs
│   │       │   └── json_export.rs
│   │       ├── filter/
│   │       │   ├── mod.rs
│   │       │   ├── ast.rs            # 过滤 DSL AST
│   │       │   ├── parser.rs         # 过滤语法解析
│   │       │   └── matcher.rs        # 过滤器匹配
│   │       └── replay/
│   │           ├── mod.rs
│   │           └── engine.rs         # 请求重放
│   │
│   ├── hook-dll/                     # 注入 DLL（Windows 专有）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                # DllMain 入口
│   │       ├── schannel.rs           # SChannel Hook
│   │       ├── winhttp.rs            # WinHTTP Hook
│   │       ├── pipe_client.rs        # Named Pipe 客户端
│   │       └── protocol.rs           # 钩子内协议序列化
│   │
│   └── flowreveal-app/               # Tauri 应用壳
│       ├── Cargo.toml                # [dependencies] engine-core, tokio, tauri, ...
│       ├── tauri.conf.json
│       ├── capabilities/
│       │   └── main.json             # Tauri v2 权限配置
│       ├── icons/
│       └── src/
│           ├── main.rs               # Tauri 入口
│           ├── lib.rs                # 模块树
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── capture.rs        # start/stop/status
│           │   ├── cert.rs           # install/uninstall CA
│           │   ├── traffic.rs        # get/list/clear requests
│           │   ├── export.rs         # export HAR/JSON
│           │   ├── replay.rs         # replay request
│           │   └── filter.rs         # apply filter
│           ├── state.rs              # AppState
│           ├── engine/               # Engine 生命周期管理
│           │   ├── mod.rs
│           │   └── manager.rs
│           └── events.rs             # Tauri event emitter
│
├── src/                              # 前端 (React + TypeScript)
│   ├── main.tsx
│   ├── App.tsx
│   ├── vite-env.d.ts
│   ├── assets/
│   │   ├── logo.svg
│   │   └── fonts/
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppShell.tsx          # 页面布局（侧边栏 + 内容区）
│   │   │   ├── Toolbar.tsx           # 顶部工具栏
│   │   │   └── StatusBar.tsx         # 底部状态栏
│   │   ├── traffic/
│   │   │   ├── TrafficList.tsx       # 请求列表
│   │   │   ├── TrafficRow.tsx        # 单行请求
│   │   │   └── TrafficTimeline.tsx   # 时间线视图
│   │   ├── detail/
│   │   │   ├── RequestDetail.tsx     # 请求详情面板
│   │   │   ├── HeadersTable.tsx      # Headers 表格
│   │   │   ├── BodyViewer.tsx        # Body 查看器（hex/text/json/image）
│   │   │   ├── CookiePanel.tsx       # Cookie 面板
│   │   │   ├── TlsInfoPanel.tsx      # TLS 信息面板
│   │   │   └── TimingBar.tsx         # 耗时瀑布图
│   │   ├── settings/
│   │   │   ├── SettingsPanel.tsx
│   │   │   ├── CertManager.tsx       # 证书管理
│   │   │   └── FilterConfig.tsx      # 过滤配置
│   │   ├── tools/
│   │   │   ├── ReplayDialog.tsx      # 重放
│   │   │   ├── ExportDialog.tsx      # 导出
│   │   │   └── SearchPanel.tsx       # 全局搜索
│   │   └── common/
│   │       ├── VirtualList.tsx       # 虚拟滚动列表
│   │       ├── JsonTree.tsx          # JSON 树形查看
│   │       ├── HexViewer.tsx         # 十六进制查看
│   │       └── ResizePanel.tsx       # 可调整面板
│   ├── hooks/
│   │   ├── useTraffic.ts
│   │   ├── useFilter.ts
│   │   ├── useSearch.ts
│   │   ├── useExport.ts
│   │   └── useCert.ts
│   ├── store/
│   │   ├── index.ts                  # Zustand store
│   │   ├── trafficSlice.ts
│   │   ├── filterSlice.ts
│   │   └── settingsSlice.ts
│   ├── types/
│   │   ├── index.ts
│   │   ├── traffic.ts
│   │   └── settings.ts
│   ├── lib/
│   │   ├── tauri-bindings.ts         # Tauri invoke 封装
│   │   └── utils.ts
│   └── styles/
│       ├── globals.css
│       ├── variables.css
│       └── theme.ts
│
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.ts                # 或 CSS Modules
├── .gitignore
├── rust-toolchain.toml               # Rust 工具链锁定
├── Cargo.toml                        # Workspace 根
└── README.md
```

### 4.2 Rust Cargo Workspace 依赖

```toml
# Cargo.toml (根 Workspace)
[workspace]
resolver = "2"
members = [
    "crates/engine-core",
    "crates/hook-dll",
    "crates/flowreveal-app",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

# crates/engine-core/Cargo.toml
[package]
name = "engine-core"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = "0.1"
bytes = "1"
lru = "0.15"

# — HTTPS / TLS —
rustls = { version = "0.23", features = ["tls12"] }
rustls-pemfile = "2"
rcgen = "0.13"
x509-parser = "0.17"
rustls-native-certs = "0.8"

# — HTTP 协议 —
hyper = { version = "1", features = ["http1", "http2", "server", "client"] }
hyper-util = { version = "0.1", features = ["tokio", "server-auto", "client-legacy"] }
http = "1"
h2 = "0.4"
tokio-tungstenite = "0.24"

# — 平台 (Windows) —
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_NetworkManagement_WindowsFilteringPlatform",
    "Win32_NetworkManagement_IpHelper",
    "Win32_Security_Cryptography_Catalog",
    "Win32_Security_Cryptography_Ssl",
    "Win32_System_Threading",
    "Win32_System_Memory",
    "Win32_System_Diagnostics_ToolHelp",
    "Win32_System_ProcessStatus",
    "Win32_System_LibraryLoader",
    "Win32_System_Console",
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_Security",
]}
detour = "0.9"
named-pipe = "0.4"

# — 平台 (Linux) —
[target.'cfg(target_os = "linux")'.dependencies]
nfqueue = "0.7"
nftnl = "0.2"

# — 存储 —
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = "0.4"

# — 日志 —
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# crates/hook-dll/Cargo.toml
[package]
name = "hook-dll"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
windows = { workspace = true }
detour = "0.9"
named-pipe = "0.4"
bincode = "2.0"     # 高效二进制序列化（Hook 内 IPC）

# crates/flowreveal-app/Cargo.toml
[package]
name = "flowreveal"
version = "0.1.0"
edition = "2024"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-process = "2"

engine-core = { path = "../engine-core" }
tokio = { workspace = true, features = ["sync", "macros"] }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 4.3 前端组件树

```
<App>
  <AppShell>
    ├── <Toolbar>
    │   ├── [Start/Stop Capture Button]
    │   ├── [Clear Button]
    │   ├── [Filter Input]
    │   └── [Settings Button]
    ├── <SplitPane>
    │   ├── <TrafficList>               # 左侧请求列表
    │   │   ├── <VirtualList>           # 虚拟滚动（万级数据不卡）
    │   │   │   └── <TrafficRow>*       # 行：Method | URL | Status | Time | Process
    │   │   └── <TimelineBar>           # 时间线滑块
    │   └── <RequestDetail>             # 右侧详情
    │       ├── <Tabs>
    │       │   ├── [Overview Tab]
    │       │   │   ├── URL, Method, Status, Duration, Process
    │       │   │   └── <TlsInfoPanel>
    │       │   ├── [Request Tab]
    │       │   │   ├── <HeadersTable>
    │       │   │   ├── <CookiePanel>
    │       │   │   └── <BodyViewer>    # Raw / Hex / JSON / Image
    │       │   ├── [Response Tab]
    │       │   │   ├── <HeadersTable>
    │       │   │   ├── <CookiePanel>
    │       │   │   └── <BodyViewer>
    │       │   ├── [WebSocket Tab]     # 仅 WS 连接
    │       │   │   └── <WebSocketView>
    │       │   │       └── <MessageFrame>*  # 每条 Frame
    │       │   └── [Timing Tab]
    │       │       └── <TimingBar>     # 瀑布图
    │       └── <ActionBar>
    │           ├── [Replay Button]
    │           └── [Export Button]
    └── <StatusBar>
        ├── Capture Status
        ├── Connection Count
        └── Data Volume
```

### 4.4 IPC 通信设计

```
Frontend (React)                        Backend (Rust/Tauri)
────────────────                        ────────────────────

invoke('start_capture', { config })
  ─────────────────────────────────────▶ commands/capture.rs
                                        → engine.start(config)
                                        → Ok(())

invoke('get_requests', { offset, limit })
  ─────────────────────────────────────▶ commands/traffic.rs
                                        → engine.get_sessions(offset, limit)
                                        ◀ Vec<HttpMessage> (JSON)

invoke('install_cert')
  ─────────────────────────────────────▶ commands/cert.rs
                                        → engine.install_ca_cert()
                                        ◀ Ok(())

invoke('export_har', { session_ids })
  ─────────────────────────────────────▶ commands/export.rs
                                        → storage::har_export::export_har()
                                        ◀ String (HAR JSON)

invoke('replay', { session_id })
  ─────────────────────────────────────▶ commands/replay.rs
                                        → replay::engine::replay()
                                        ◀ Ok(HttpMessage)  # 重放后的新响应

// 实时推送（Backend → Frontend）
engine.event_tx ─▶ Tauri Window.emit("traffic:request", HttpMessage)
                                      ─────────────────▶ listen('traffic:request', cb)
                                                          → UI 追加新行

engine.event_tx ─▶ Tauri Window.emit("traffic:stats", EngineStats)
                                      ─────────────────▶ listen('traffic:stats', cb)
                                                          → StatusBar 更新
```

---

## 5. 潜在风险与缓解措施

### 5.1 WFP 相关风险

| 风险                               | 概率 | 影响 | 缓解措施                                                        |
|------------------------------------|------|------|----------------------------------------------------------------|
| WFP REDIRECT 规则注册失败           | 低   | 高   | 1. 使用动态 Session（进程退出自动清理）<br>2. 注册全局异常 handler 做回滚<br>3. 启动时检测 WFP 服务状态 |
| 代理自身流量被重定向（死循环）       | 中   | 极高 | 1. 在过滤规则中使用 `FWPM_CONDITION_ALE_PACKAGE_ID` 排除自身 SID<br>2. 同时使用 `FWPM_CONDITION_FLAGS & FWP_CONDITION_FLAG_IS_LOOPBACK` 忽略回环流量<br>3. 代理绑定到 127.0.0.1，回环流量自动跳过 REDIRECT 层 |
| 与其他安全软件（杀软/防火墙）冲突   | 高   | 中   | 1. SubLayer 的 weight 设置适当值（不要最低/最高）<br>2. 提示用户将 FlowReveal 加入杀软白名单<br>3. 检测冲突时降级为正向代理模式 |
| 管理员权限需求                     | 总是 | 低   | 1. 启动时检测权限，自动请求 UAC 提权<br>2. 提供 "无 WFP 的降级模式"（仅正向代理） |
| Windows 版本兼容性（REDIRECT 层）   | 低   | 高   | 1. `FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4` 仅 Windows 8+ 可用<br>2. Windows 7 不支持（已 EOL，可忽略）<br>3. 启动时检测 OS 版本，不支持时降级为正向代理 |

### 5.2 API Hook 相关风险

| 风险                       | 概率 | 影响 | 缓解措施                                                        |
|----------------------------|------|------|----------------------------------------------------------------|
| Hook DLL 注入导致目标进程崩溃| 高   | 高   | 1. Hook 函数内不能 panic，全部用 `catch_unwind` 包裹<br>2. 极小化 DLL 依赖（静态链接 Windows CRT）<br>3. 设置超时，超时则跳过该进程 |
| 杀软拦截 DLL 注入           | 高   | 中   | 1. 使用 SetWindowsHookEx（合法的 Windows 机制）<br>2. 不用 LoadLibrary 远程注入，用 Windows 消息钩子<br>3. 签名的 DLL 可降低被拦截概率 |
| 64 位系统 PatchGuard + 杀软防护 | 高 | 高 | 1. SChannel.dll 属于系统 DLL，内联 Hook 容易触发 PatchGuard 崩溃或被杀软拦截<br>2. **推荐策略**：优先只 Hook WinHTTP.dll / WinINet.dll（应用层，风险更低）<br>3. SChannel Hook 仅作为高级可选功能 |
| 64 位 Hook 注入 32 位进程   | 中   | 中   | 1. 编译两个版本的 hook-dll（x86 / x64）<br>2. 根据目标进程架构选择注入的 DLL |
| Detours 与其他 Hook 冲突    | 低   | 中   | 1. 检查函数头是否已被修改<br>2. 支持 Detours Chain 模式<br>3. 优先级：只做观察者，不修改数据流 |
| Windows 11 SChannel 实现变化  | 中 | 高 | 1. Windows 11 的 SChannel 实现可能变更，Hook 点需持续维护<br>2. 选择 Hook WinHTTP/WinINet 作为稳定入口点<br>3. 定期跟进 Windows Insider Build 验证兼容性<br>4. MITM 解密提供明文，API Hook 在此链路上是锦上添花，非必需 |

**SChannel vs WinHTTP Hook 策略建议**：

| 方案                 | 风险 | 稳定性 | 覆盖范围 | 建议       |
|----------------------|------|--------|----------|-----------|
| WinHTTP/WinINet Hook | 低   | 高     | 使用 WinHTTP 的应用（浏览器、.NET、curl 等） | ★ 阶段 4 试点 |
| SChannel Hook        | 高   | 中     | 所有基于 SChannel 的 TLS 应用     | 视稳定性再决定 |

### 5.3 MITM / 证书相关风险

| 风险                       | 概率 | 影响 | 缓解措施                                                        |
|----------------------------|------|------|----------------------------------------------------------------|
| 证书固定导致 App 拒绝连接    | 高   | 中   | 1. "Bypass MITM" 白名单<br>2. Frida/Objection Hook 绕过方案（高级）|
| 自签名 CA 私钥泄露          | 低   | 极高 | 1. 私钥仅存本地，使用 ACL 保护<br>2. 每次安装生成唯一 CA<br>3. 应用关闭时可选删除 CA |
| TLS 版本不兼容              | 中   | 中   | 1. rustls 支持 TLS 1.2/1.3<br>2. 自动协商最高版本<br>3. 遇到不兼容时记录到日志，透传连接 |
| 证书链验证失败              | 低   | 低   | 1. 使用 `rustls-native-certs` 加载系统根证书<br>2. 严格验证上游证书（不信任自签名上游） |

### 5.4 性能风险

| 风险                   | 概率 | 影响 | 缓解措施                                                        |
|------------------------|------|------|----------------------------------------------------------------|
| WFP REDIRECT 增加网络延迟 | 总是 | 低   | 1. REDIRECT 层由系统内核高效处理，延迟极低<br>2. 协议解析在代理内异步完成，不阻塞重定向路径 |
| 内存泄漏（长时间捕获）  | 中   | 高   | 1. LRU 淘汰 + 自动写入 SQLite<br>2. 设置 `max_body_size` 截断<br>3. 定期 GC 检查 |
| UI 卡顿（万级请求）     | 中   | 中   | 1. 前端使用虚拟滚动（react-window）<br>2. 分页加载（每次 50 条）<br>3. Web Worker 做搜索索引 |

### 5.5 法律合规风险

| 风险                       | 缓解措施                                                        |
|----------------------------|----------------------------------------------------------------|
| 中间人攻击工具的合规性      | 1. 应用启动时弹窗 EULA 声明仅用于合法调试目的<br>2. 检测是否在非开发者环境中运行<br>3. 明确提示 "安装根证书可能被安全软件视为威胁" |
| 反病毒软件误报              | 1. 提交样本到 VirusTotal 白名单<br>2. 获取 Code Signing Certificate<br>3. 联系主流 AV 厂商做白名单认证 |
| GDPR / 隐私合规             | 1. 所有数据本地存储，不上传<br>2. 会话关闭后可选清除所有捕获数据<br>3. 隐私政策中说明 "此工具仅捕获调试目标应用流量" |

---

## 附录 A：关键 Windows API 与库速查

| 用途                        | API / Library                           |
|-----------------------------|-----------------------------------------|
| WFP 用户态引擎               | `fwpuclnt.dll` → `FwpmEngineOpen`, `FwpmFilterAdd` |
| WFP 重定向层（系统内置）      | `FWPM_LAYER_ALE_AUTH_CONNECT_REDIRECT_V4`（纯用户态，无需 .sys） |
| TCP 连接表查询               | `iphlpapi.dll` → `GetExtendedTcpTable` |
| ETW TCP/IP 实时事件          | `Microsoft-Windows-TCPIP` 提供者 → TraceLogging 订阅 |
| 进程信息查询                 | `kernel32.dll` → `OpenProcess`, `QueryFullProcessImageNameW` |
| 证书存储操作                 | `crypt32.dll` → `CertOpenStore`, `CertAddCertificateContextToStore` |
| DLL 注入                     | `kernel32.dll` → `CreateRemoteThread` + `LoadLibraryW` |
| API Hook                     | Microsoft Detours (detours library)      |
| Named Pipe IPC               | `kernel32.dll` → `CreateNamedPipeW`, `ConnectNamedPipe` |
| Token 权限提升               | `advapi32.dll` → `AdjustTokenPrivileges` (SE_DEBUG_NAME) |

## 附录 B：推荐的 Rust Crate 替代方案

| 用途            | 首选                  | 备选                      |
|-----------------|----------------------|---------------------------|
| TLS             | rustls + tokio-rustls| native-tls (schannel)     |
| HTTP/1.1 代理   | hyper 1.x            | reqwest (仅客户端)         |
| HTTP/2          | h2                   | hyper 内置 h2              |
| WebSocket       | tokio-tungstenite    | tungstenite               |
| 证书生成         | rcgen                | openssl (需外部依赖)        |
| 异步运行时       | tokio                | async-std                 |
| 日志             | tracing              | log + env_logger          |
| 序列化           | serde + serde_json   | —                         |
| SQLite           | rusqlite (bundled)   | sqlx                      |
| JSON 导出 (前端) | —                    | file-saver (npm)           |
| 虚拟滚动 (前端)  | react-window         | @tanstack/virtual         |
| 状态管理 (前端)  | zustand              | jotai / redux-toolkit     |
| UI 组件 (前端)   | shadcn/ui + Tailwind  | antd / mui                |

---

> **本文档版本**: v1.0  
> **适用目标**: FlowReveal 从零开始的第一版完整开发规划  
> **预计总工作量**: 6 阶段 × 约 4-6 周/阶段 = 约 24-36 周（单人全职）  
> **后续迭代方向**: HTTP/3 (QUIC) 支持、gRPC-web 解析、GraphQL 语义分析、自动漏洞检测、流量 diff 对比