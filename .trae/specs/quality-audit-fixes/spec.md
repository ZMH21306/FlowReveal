# FlowReveal全面质量审查 Spec

## Why
项目核心功能已开发完成，但存在多个不符合初始目标的问题：UI按钮无响应（已修复PacketCaptured事件连接和UI线程调度）、网卡识别错误（已修复虚拟适配器过滤）、大量功能模块未实现（WFP引擎、内容查看器、设置界面、请求编辑重发等）、缺少单元测试、存在代码质量问题。需要系统性审查并修复所有不符合项。

## What Changes
- 修复DataGrid数据绑定问题（Conversations vs FilteredConversations）
- 修复搜索结果高亮UI实现
- 实现HAR格式导出
- 实现请求编辑重发功能
- 实现错误会话自动高亮（4xx/5xx红色标记）
- 修复代码编译警告（CA1416平台兼容性）
- 添加核心模块单元测试
- 修复TcpReassembler中潜在的内存泄漏
- 修复HttpsProxyServer中数据中继未正确解析HTTP的问题
- 修复ProtocolParser中TCP流数据未正确消费的竞态条件

## Impact
- Affected specs: UI交互、数据导出、协议解析、HTTPS代理
- Affected code: MainWindowViewModel.cs, MainWindow.axaml, SessionStore.cs, ProtocolParser.cs, TcpReassembler.cs, HttpsProxyServer.cs

## ADDED Requirements

### Requirement: DataGrid数据绑定修正
DataGrid SHALL绑定到FilteredConversations而非Conversations，确保过滤功能正常工作。

#### Scenario: 过滤后列表更新
- **WHEN** 用户应用过滤条件
- **THEN** DataGrid仅显示匹配的会话，而非全部会话

### Requirement: 错误会话自动高亮
DataGrid中4xx/5xx状态码的会话SHALL以红色文字显示，慢请求SHALL以黄色文字显示。

#### Scenario: 错误会话标记
- **WHEN** 会话响应状态码为4xx或5xx
- **THEN** 该行以红色文字显示

### Requirement: HAR格式导出
系统SHALL支持HAR（HTTP Archive）格式导出。

#### Scenario: HAR导出
- **WHEN** 用户选择HAR格式导出
- **THEN** 生成符合HAR 1.2规范的JSON文件

### Requirement: 请求编辑重发
系统SHALL支持编辑已捕获的请求并重新发送。

#### Scenario: 请求重发
- **WHEN** 用户选择某条请求并点击重发
- **THEN** 打开编辑器，允许修改后发送请求并展示响应

### Requirement: 核心模块单元测试
系统SHALL包含核心模块的单元测试，覆盖率≥70%。

#### Scenario: 单元测试执行
- **WHEN** 运行dotnet test
- **THEN** 所有测试通过，核心模块覆盖率≥70%

## MODIFIED Requirements

### Requirement: 搜索结果高亮
搜索结果SHALL在UI中以高亮颜色显示匹配文本，而非仅在SearchMatch中记录位置。

### Requirement: HTTPS代理数据解析
HttpsProxyServer的CONNECT隧道SHALL正确解析中继数据为HttpConversation并通过事件通知。

## REMOVED Requirements
（无移除项）
