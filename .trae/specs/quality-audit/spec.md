# FlowReveal 全面质量审查 Spec

## Why
FlowReveal项目核心功能开发已基本完成，但对照初始spec.md中的需求定义和checklist.md中的验收标准，仍存在多项不符合项。需要执行系统性质量审查，识别所有缺陷并制定修复计划，确保项目达标。

## What Changes
- 修复已识别的代码缺陷（UI线程安全、事件连接、适配器识别等）
- 补全缺失功能模块（内容查看器、设置界面、搜索高亮、错误高亮等）
- 修复编译警告（CA1416平台兼容性、CS0067未使用事件等）
- 添加关键单元测试（TCP重组、HTTP解析、过滤引擎等核心模块）
- 修复运行时问题（中文乱码、DataGrid数据绑定、抓包数据流完整性）

## Impact
- Affected specs: implement-windows-packet-capture/spec.md 全部需求
- Affected code: 几乎所有模块均需审查和修复

## ADDED Requirements

### Requirement: 代码质量审查
系统SHALL通过以下代码质量检查：
- 零编译错误
- 编译警告数量最小化（CA1416需添加平台守卫，CS0067需实现或移除）
- 无空引用风险（nullable正确标注）
- 无资源泄漏（IDisposable正确实现）

#### Scenario: 编译警告修复
- **WHEN** 执行dotnet build
- **THEN** 编译警告数量应≤5个（仅保留不可避免的第三方包警告）

### Requirement: 运行时功能验证
系统SHALL在实际运行中满足以下条件：
- 点击Start Capture按钮后开始抓包，状态栏显示"Capturing"
- 抓包期间DataGrid实时显示HTTP会话
- 点击Stop Capture按钮后停止抓包，状态栏显示"Stopped"
- 点击某条会话后详情面板显示请求/响应内容
- 过滤功能正常工作
- 中文内容正确显示（无乱码）

#### Scenario: 完整抓包流程
- **WHEN** 用户启动抓包→浏览网页→停止抓包
- **THEN** DataGrid中显示HTTP会话列表，点击可查看详情

### Requirement: 核心模块单元测试
系统SHALL为核心模块提供单元测试，覆盖率≥80%：
- TCP流重组（含乱序场景）
- HTTP协议解析（含边界条件）
- 内容解码（gZip/Deflate/Chunked）
- 过滤引擎（各运算符和组合逻辑）
- IP包解析

#### Scenario: 单元测试通过
- **WHEN** 执行dotnet test
- **THEN** 所有测试通过，核心模块覆盖率≥80%

## MODIFIED Requirements

### Requirement: 搜索结果高亮
搜索结果SHALL在UI中通过视觉标记（背景色变化或文字加粗）高亮显示匹配位置，而非仅在SearchMatch中记录Position/Context。

### Requirement: 错误自动高亮
DataGrid中4xx/5xx状态码的行SHALL通过红色文字或红色背景自动高亮标记，慢请求SHALL通过黄色标记。

## REMOVED Requirements

### Requirement: WFP抓包引擎
**Reason**: 用户已确认WFP引擎后续补全，当前版本仅使用原始套接字
**Migration**: 保留CaptureEngineType.Wfp枚举值和接口定义，Task 6标记为后续实施
