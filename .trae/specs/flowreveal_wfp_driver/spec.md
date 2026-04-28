# FlowReveal — 网络调试工具（WFP 内核驱动方案）
# 产品需求文档 (PRD)

## Overview
- **Summary**: 基于 Windows 过滤平台 (WFP) 内核 Callout 驱动 + 本地 MITM 代理的网络调试工具，实现系统级 HTTP/HTTPS 流量的明文捕获与展示。
- **Purpose**: 解决现有工具在捕获不可绕过性、进程关联能力及协议覆盖范围上的不足，为开发者和安全人员提供更强大的网络调试能力。
- **Target Users**: 网络安全工程师、Web 开发者、系统管理员、网络调试人员。

## Goals
- **G1**: 实现系统级 HTTP/HTTPS 流量的透明捕获，无应用可绕过
- **G2**: 完整解析并展示 HTTP/HTTPS 请求/响应的头、体及关键元数据
- **G3**: 提供基于表格的 UI，支持排序、筛选、搜索操作
- **G4**: 实现 HTTPS 流量的 MITM 解密与明文展示
- **G5**: 关联网络流量与发起进程，支持进程级过滤
- **G6**: 支持 Windows 10+ 平台，无外部库依赖

## Non-Goals (Out of Scope)
- **NG1**: 不支持 Windows 7 及更早版本
- **NG2**: 不支持非 HTTP 协议的深度解析（仅捕获原始流）
- **NG3**: 不支持 HTTP/2 和 QUIC (HTTP/3) 协议的深度解析
- **NG4**: 不支持跨平台（仅 Windows 平台）
- **NG5**: 不提供网络流量修改或拦截能力（仅捕获与展示）

## Background & Context
- **技术栈**: C# (.NET 10.0) + Avalonia UI + WFP 内核驱动 (C/WDK)
- **架构选择**: 方案 C — WFP 内核驱动 + 用户态服务 + 本地 MITM 代理的混合架构
- **核心挑战**: 内核驱动开发门槛高、HTTPS 解密复杂性、WFP 与代理的协同工作

## Functional Requirements
- **FR-1**: 系统级网络流量捕获
  - 通过 WFP 内核驱动在 Stream 层拦截 TCP 流量
  - 重定向 80/443 端口流量到本地代理进行 HTTP 解析
  - 捕获并传递非 80/443 端口的 TCP 原始流

- **FR-2**: HTTP/HTTPS 协议解析
  - 解析 HTTP/1.1 请求头、响应头、请求体、响应体
  - 对 HTTPS 流量进行 MITM 解密并解析
  - 支持常见 HTTP 方法（GET、POST、PUT、DELETE、PATCH）

- **FR-3**: 表格 UI 展示
  - 表格包含：请求时间戳、方法、URL、状态码、响应时间、请求大小、响应大小、进程名
  - 支持按各列排序、筛选、搜索
  - 点击行展示请求详情（头、体、元数据）

- **FR-4**: 进程关联与过滤
  - 关联网络流量与发起进程（ID、名称、路径）
  - 支持按进程名/ID 筛选流量

- **FR-5**: 证书管理
  - 生成自签名根 CA 证书
  - 动态生成目标域名的 TLS 证书
  - 管理证书导入与信任

- **FR-6**: 性能与可靠性
  - 支持高并发网络流量（≥100 QPS）
  - 大文件传输时的流式处理与截断
  - 优雅处理网络异常与崩溃恢复

## Non-Functional Requirements
- **NFR-1**: 安全性
  - 根 CA 私钥安全存储（DPAPI 加密）
  - 内核驱动符合安全最佳实践
  - 最小权限原则（仅管理员权限下运行）

- **NFR-2**: 性能
  - 内核驱动零拷贝设计
  - 用户态服务异步 I/O 处理
  - 表格 UI 虚拟化，支持 10000+ 条日志的流畅滚动

- **NFR-3**: 可靠性
  - 内核驱动异常不导致系统崩溃（BSOD）
  - 服务自动重启机制
  - 日志持久化（可选）

- **NFR-4**: 可维护性
  - 模块化架构，清晰的分层设计
  - 完善的日志与监控
  - 代码注释与文档

- **NFR-5**: 兼容性
  - 支持 Windows 10 1607+、Windows 11、Windows Server 2016+
  - 兼容 32 位与 64 位系统

## Constraints
- **Technical**: 
  - .NET 10.0 运行时（不支持 Windows 7）
  - WDK 10+ 开发环境
  - 内核驱动需要 Microsoft 签名（Attestation Signing 或 HLK 认证）

- **Business**: 
  - 开发周期：核心功能 8-10 周
  - 驱动签名成本：$200-400/年（EV 代码签名证书）

- **Dependencies**: 
  - Windows Filtering Platform (WFP) API
  - Windows Driver Kit (WDK)
  - System.Net.Sockets、System.Security.Cryptography（.NET 内置）
  - Avalonia UI（无外部库依赖）

## Assumptions
- **A1**: 用户具备管理员权限运行本应用
- **A2**: 用户接受安装自签名根 CA 证书到受信任存储区
- **A3**: 开发环境具备 WDK 10+ 与 Visual Studio 2022+
- **A4**: 内核驱动签名通过 Microsoft Attestation Signing 流程

## Acceptance Criteria

### AC-1: 系统级流量捕获
- **Given**: 应用以管理员身份运行，驱动已安装
- **When**: 任何应用发起 HTTP/HTTPS 请求
- **Then**: 流量被捕获并在 UI 中显示，无应用可绕过
- **Verification**: `programmatic`
- **Notes**: 验证方法：使用 curl、浏览器、自定义应用分别发起请求，确认均被捕获

### AC-2: HTTPS 解密展示
- **Given**: 根 CA 证书已导入受信任存储区
- **When**: 访问 HTTPS 网站（如 https://example.com）
- **Then**: UI 中显示解密后的 HTTP 明文内容（请求头、响应头、体）
- **Verification**: `programmatic`
- **Notes**: 验证方法：对比捕获的内容与浏览器开发者工具中的内容

### AC-3: 表格 UI 功能
- **Given**: 应用运行并捕获了多条网络请求
- **When**: 用户点击列头、输入搜索关键词、选择筛选条件
- **Then**: 表格正确排序、筛选、搜索，且响应时间 < 300ms
- **Verification**: `human-judgment`
- **Notes**: 验证方法：手动测试排序、筛选、搜索功能的正确性与响应速度

### AC-4: 进程关联
- **Given**: 不同进程（如 Chrome、curl、自定义应用）发起网络请求
- **When**: 查看捕获的日志
- **Then**: 每条日志显示正确的进程名与 ID
- **Verification**: `programmatic`
- **Notes**: 验证方法：通过进程名筛选，确认只显示对应进程的流量

### AC-5: 性能与稳定性
- **Given**: 应用持续运行并处理 100 QPS 的网络请求
- **When**: 运行 30 分钟
- **Then**: CPU 使用率 < 10%，内存增长 < 100MB，无崩溃
- **Verification**: `programmatic`
- **Notes**: 验证方法：使用压测工具（如 Apache Bench）模拟高并发请求

### AC-6: 内核驱动安全性
- **Given**: 驱动已安装并运行
- **When**: 系统运行多种网络应用
- **Then**: 无 BSOD，无系统不稳定现象
- **Verification**: `human-judgment`
- **Notes**: 验证方法：长期运行测试，观察系统稳定性

## Open Questions
- [ ] **Q1**: 内核驱动签名方案选择（Attestation Signing vs HLK 认证）
- [ ] **Q2**: 非 80/443 端口的 TCP 流量是否需要展示，以及如何展示
- [ ] **Q3**: 大文件传输的截断策略（默认截断大小）
- [ ] **Q4**: 日志持久化存储方案（文件格式、轮转策略）
- [ ] **Q5**: 与第三方安全软件（如杀毒软件）的兼容性处理