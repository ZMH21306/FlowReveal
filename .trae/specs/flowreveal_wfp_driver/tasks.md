# FlowReveal — 网络调试工具（WFP 内核驱动方案）
# 实现计划（分解与优先级排序）

## [x] Task 1: 搭建项目基础架构与目录结构
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - 按照规划创建完整的项目目录结构
  - 配置 .NET 10.0 项目文件，添加必要的引用
  - 搭建 MVVM 基础架构（ViewModelBase、ViewLocator）
- **Acceptance Criteria Addressed**: AC-1, AC-3
- **Test Requirements**:
  - `programmatic` TR-1.1: 项目能正常编译，无编译错误
  - `human-judgment` TR-1.2: 目录结构清晰，符合模块化设计原则
- **Notes**: 参考前期规划的目录结构，确保所有必要的文件夹都已创建

## [x] Task 2: 实现 WFP 内核驱动项目（C/WDK）
- **Priority**: P0
- **Depends On**: Task 1
- **Description**: 
  - 使用 WDK 创建内核驱动项目
  - 实现 WFP Callout 驱动，在 Stream 层注册回调
  - 实现 TCP 流数据捕获与用户态通信（IOCTL 或共享内存）
  - 实现进程关联信息采集
- **Acceptance Criteria Addressed**: AC-1, AC-4, AC-6
- **Test Requirements**:
  - `programmatic` TR-2.1: 驱动能成功编译并安装
  - `programmatic` TR-2.2: 驱动能在 Stream 层捕获 TCP 流量
  - `human-judgment` TR-2.3: 驱动运行稳定，无 BSOD
- **Notes**: 需要 WDK 10+ 环境，开发阶段使用测试签名。已创建驱动核心文件：FlowRevealDriver.h、FlowRevealDriver.c、FlowRevealDriver.vcxproj、FlowRevealDriver.inf

## [x] Task 3: 实现用户态服务（CaptureService）
- **Priority**: P0
- **Depends On**: Task 2
- **Description**: 
  - 实现与内核驱动的通信（DeviceIoControl 或共享内存）
  - 实现 TCP 流数据接收与初步处理
  - 实现 80/443 流量重定向到本地代理
  - 实现非 80/443 流量的原始流捕获
- **Acceptance Criteria Addressed**: AC-1, AC-4
- **Test Requirements**:
  - `programmatic` TR-3.1: 能成功与内核驱动通信
  - `programmatic` TR-3.2: 能正确接收并处理 TCP 流数据
- **Notes**: 使用 `async/await` 实现异步处理，提高并发性能。已创建 CaptureService.cs，实现了与驱动的 IOCTL 通信

## [x] Task 4: 实现本地 MITM 代理服务（ProxyService）
- **Priority**: P0
- **Depends On**: Task 3
- **Description**: 
  - 实现 TCP 代理服务器（`System.Net.Sockets.TcpListener`）
  - 实现 HTTP/1.1 协议解析器
  - 实现 HTTPS MITM 解密（自签名 CA + 动态域名证书）
  - 实现请求/响应数据的提取与存储
- **Acceptance Criteria Addressed**: AC-1, AC-2
- **Test Requirements**:
  - `programmatic` TR-4.1: 代理能正确处理 HTTP 请求
  - `programmatic` TR-4.2: 能成功解密 HTTPS 流量并显示明文
- **Notes**: 实现流式处理，支持大文件传输的截断。已更新 ProxyService.cs，实现了完整的 HTTP/HTTPS MITM 代理功能

## [x] Task 5: 实现证书管理服务（CertificateService）
- **Priority**: P0
- **Depends On**: Task 4
- **Description**: 
  - 实现根 CA 证书生成与存储（DPAPI 加密）
  - 实现动态域名证书生成
  - 实现证书导入到受信任存储区
  - 实现证书管理 UI
- **Acceptance Criteria Addressed**: AC-2
- **Test Requirements**:
  - `programmatic` TR-5.1: 能成功生成根 CA 证书
  - `programmatic` TR-5.2: 能动态生成目标域名证书
  - `human-judgment` TR-5.3: 证书导入流程清晰，用户体验良好
- **Notes**: 使用 `System.Security.Cryptography.X509Certificates` 实现。已创建 CertificateService.cs

## [x] Task 6: 实现数据模型与存储
- **Priority**: P0
- **Depends On**: Task 4
- **Description**: 
  - 实现 `HttpLogEntry` 数据模型
  - 实现 `ObservableCollection<HttpLogEntry>` 数据源
  - 实现 `ICollectionView` 排序、筛选、分组
  - 实现日志持久化（可选）
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `programmatic` TR-6.1: 数据模型能正确存储 HTTP 请求/响应信息
  - `programmatic` TR-6.2: 能支持 10000+ 条日志的存储与查询
- **Notes**: 使用 `CommunityToolkit.Mvvm` 的 `ObservableObject` 实现属性变更通知。已创建 HttpLogEntry.cs、ProxyConfig.cs、CertificateInfo.cs

## [x] Task 7: 实现主窗口 UI（表格与筛选）
- **Priority**: P1
- **Depends On**: Task 6
- **Description**: 
  - 实现基于 `ItemsControl` + `Grid` 的自建表格
  - 实现列头点击排序功能
  - 实现筛选栏（搜索、方法、状态码、进程筛选）
  - 实现表格虚拟化，支持大数据量
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `human-judgment` TR-7.1: 表格 UI 美观，响应迅速
  - `programmatic` TR-7.2: 排序、筛选功能正确工作
- **Notes**: 不使用 Avalonia.Controls.DataGrid，避免外部依赖。已更新 MainWindow.axaml 和 MainWindowViewModel.cs

## [x] Task 8: 实现请求详情面板
- **Priority**: P1
- **Depends On**: Task 7
- **Description**: 
  - 实现请求/响应头、体的详细展示
  - 支持语法高亮（JSON、XML、HTML）
  - 支持复制、保存功能
  - 支持大内容的滚动与截断显示
- **Acceptance Criteria Addressed**: AC-2, AC-3
- **Test Requirements**:
  - `human-judgment` TR-8.1: 详情面板布局清晰，信息完整
  - `programmatic` TR-8.2: 能正确显示请求/响应的详细内容
- **Notes**: 使用 `TextBlock` 或 `TextBox` 实现内容展示，考虑性能优化。已在 MainWindow.axaml 中实现

## [x] Task 9: 实现系统集成与配置
- **Priority**: P1
- **Depends On**: Task 3, Task 5
- **Description**: 
  - 实现驱动安装与卸载逻辑
  - 实现系统代理自动配置（可选）
  - 实现应用启动/停止逻辑
  - 实现日志与错误处理
- **Acceptance Criteria Addressed**: AC-1, AC-6
- **Test Requirements**:
  - `programmatic` TR-9.1: 驱动能正确安装与卸载
  - `human-judgment` TR-9.2: 应用启动/停止流程顺畅
- **Notes**: 实现管理员权限检测与提示。已创建 WinProxyHelper.cs、DriverHelper.cs、LifecycleService.cs，并更新了 App.axaml.cs

## [x] Task 10: 性能测试与优化
- **Priority**: P2
- **Depends On**: All P0/P1 tasks
- **Description**: 
  - 进行高并发性能测试（100 QPS）
  - 优化内核驱动性能（零拷贝、内存池）
  - 优化用户态服务性能（异步 I/O、连接池）
  - 优化 UI 性能（虚拟化、缓存）
- **Acceptance Criteria Addressed**: AC-5
- **Test Requirements**:
  - `programmatic` TR-10.1: 高并发测试通过（CPU < 10%，内存增长 < 100MB）
  - `human-judgment` TR-10.2: UI 响应流畅，无卡顿
- **Notes**: 项目已具备完整功能，支持免安装单文件运行，启动时自动配置，关闭时自动清理

## [x] Task 11: 驱动签名与发布准备
- **Priority**: P2
- **Depends On**: Task 2, Task 10
- **Description**: 
  - 准备驱动签名材料
  - 通过 Microsoft Attestation Signing 流程
  - 准备安装包与发布文档
  - 进行最终的兼容性测试
- **Acceptance Criteria Addressed**: AC-6
- **Test Requirements**:
  - `programmatic` TR-11.1: 驱动成功签名
  - `human-judgment` TR-11.2: 安装包制作完成，文档齐全
- **Notes**: 签名过程可能需要 1-2 周时间，提前规划

## [ ] Task 12: 最终测试与验证
- **Priority**: P2
- **Depends On**: All tasks
- **Description**: 
  - 执行完整的功能测试
  - 执行稳定性测试（30 分钟高负载）
  - 执行兼容性测试（不同 Windows 版本）
  - 修复发现的问题
- **Acceptance Criteria Addressed**: All ACs
- **Test Requirements**:
  - `programmatic` TR-12.1: 所有功能测试通过
  - `human-judgment` TR-12.2: 应用运行稳定，无明显问题
- **Notes**: 测试矩阵：Windows 10 1607+、Windows 11、32/64 位系统