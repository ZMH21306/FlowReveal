# FlowReveal HTTP Debugger - Product Requirement Document

## Overview
- **Summary**: 基于 Avalonia UI 的 Windows 平台网络调试工具，实现 HTTP/HTTPS 协议数据的明文捕获与展示，提供表格化流量列表、排序筛选搜索功能，并支持请求/响应详情查看。
- **Purpose**: 解决开发者在调试网络请求时无法完整监控系统级流量的痛点，提供类似 Fiddler/Charles 的网络分析能力，且无需依赖系统代理配置。
- **Target Users**: 软件开发者、测试工程师、API 调试人员

## Goals
- 实现系统级 HTTP/HTTPS 流量捕获（无应用可绕过）
- 提供表格化流量列表，包含请求时间戳、方法、URL、状态码、响应时间、请求/响应大小
- 支持按各列排序、全局搜索、筛选功能
- 提供请求/响应详情视图（Headers、Body、Raw、Timing）
- 支持 HAR 格式导出

## Non-Goals (Out of Scope)
- 不支持 HTTP/2 协议（首期）
- 不支持修改请求/响应数据（首期）
- 不支持移动端网络监控
- 不提供跨平台支持（专注 Windows 10+）

## Background & Context
- 现有模板基于 Avalonia UI 11.3.12 + .NET 10 + CommunityToolkit.Mvvm
- 要求不依赖外部库、不使用系统代理
- 目标平台：Windows 10 及以上版本

## Functional Requirements
- **FR-1**: 捕获系统级 HTTP/HTTPS 流量，包括 localhost 连接
- **FR-2**: 解析并展示请求头、响应头、请求体、响应体
- **FR-3**: 表格展示流量列表，包含：请求时间戳、请求方法、URL、状态码、响应时间、请求大小、响应大小
- **FR-4**: 支持按各列排序（升序/降序）
- **FR-5**: 支持全局搜索（搜索 URL、请求体、响应体）
- **FR-6**: 支持按状态码、请求方法筛选
- **FR-7**: 支持 HAR 格式导出

## Non-Functional Requirements
- **NFR-1**: 支持 Windows 10 及以上版本（x64/ARM64）
- **NFR-2**: 捕获性能：在 100 Mbps 带宽下 CPU 占用 < 20%
- **NFR-3**: 内存占用：10000 条记录时 < 200 MB
- **NFR-4**: 响应时间：表格滚动流畅，无明显卡顿
- **NFR-5**: 安全性：HTTPS 解密使用用户可控的 CA 证书

## Constraints
- **Technical**: .NET 10, Avalonia UI 11.3.12, 不依赖外部库
- **Business**: 无需 EV 代码签名证书（避免驱动方案）
- **Dependencies**: Windows WFP API, .NET BCL

## Assumptions
- 用户接受安装自定义 CA 证书以解密 HTTPS 流量
- 用户具备管理员权限运行应用
- 用户使用支持 TLS 的现代浏览器和应用

## Acceptance Criteria

### AC-1: HTTP 流量捕获
- **Given**: 应用以管理员权限运行
- **When**: 启动捕获并发起 HTTP 请求（如浏览器访问 http://example.com）
- **Then**: 请求出现在流量列表中，包含完整的请求/响应数据
- **Verification**: `programmatic`
- **Notes**: 测试用 HTTPBin 等公开 API

### AC-2: HTTPS 流量解密
- **Given**: 用户已安装应用根 CA 证书，应用以管理员权限运行
- **When**: 启动捕获并发起 HTTPS 请求（如浏览器访问 https://example.com）
- **Then**: 请求出现在流量列表中，响应体显示为明文
- **Verification**: `programmatic`
- **Notes**: 使用非证书固定的网站测试

### AC-3: 表格列完整性
- **Given**: 流量列表中有捕获的请求
- **When**: 查看表格列
- **Then**: 显示时间戳、方法、URL、状态码、响应时间、请求大小、响应大小
- **Verification**: `human-judgment`

### AC-4: 排序功能
- **Given**: 流量列表中有多条记录
- **When**: 点击表格列标题
- **Then**: 列表按该列升序/降序排序，列标题显示排序指示
- **Verification**: `human-judgment`

### AC-5: 搜索功能
- **Given**: 流量列表中有多条记录
- **When**: 在搜索框输入关键词
- **Then**: 仅显示包含该关键词的记录（匹配 URL、请求体、响应体）
- **Verification**: `programmatic`

### AC-6: 筛选功能
- **Given**: 流量列表中有多条记录
- **When**: 选择筛选条件（如状态码 2xx）
- **Then**: 仅显示符合条件的记录
- **Verification**: `programmatic`

### AC-7: 详情查看
- **Given**: 点击某条流量记录
- **When**: 查看详情面板
- **Then**: 显示请求头、响应头、请求体、响应体、原始数据、时序信息
- **Verification**: `human-judgment`

### AC-8: HAR 导出
- **Given**: 流量列表中有多条记录
- **When**: 选择导出 HAR 格式
- **Then**: 生成符合 HAR 1.2 规范的 JSON 文件
- **Verification**: `programmatic`
- **Notes**: 可导入 Chrome DevTools 验证

### AC-9: 性能要求
- **Given**: 持续捕获 10000 条 HTTP 请求
- **When**: 监控系统资源使用
- **Then**: CPU 占用 < 20%，内存 < 200 MB
- **Verification**: `programmatic`

## Open Questions
- [ ] 是否需要支持请求重放功能？
- [ ] 是否需要支持请求修改功能？