# FlowReveal HTTP Debugger - Implementation Plan

## [x] Task 1: 创建核心数据模型
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - 创建 `HttpTrafficRecord` 模型类（时间戳、方法、URL、状态码、响应时间、请求/响应大小）
  - 创建 `HttpHeader` 模型类
  - 创建 `CaptureSession` 模型类
- **Acceptance Criteria Addressed**: AC-1, AC-2, AC-3
- **Test Requirements**:
  - `programmatic` TR-1.1: 模型类可序列化/反序列化
  - `human-judgment` TR-1.2: 模型属性完整，命名规范
- **Notes**: 使用 CommunityToolkit.Mvvm 的 ObservableObject

## [x] Task 2: 实现 HTTP/1.1 协议解析器
- **Priority**: P0
- **Depends On**: Task 1
- **Description**: 
  - 实现 HTTP 请求/响应解析器
  - 支持分块传输编码（Transfer-Encoding: chunked）
  - 支持 Content-Length 定界
  - 支持 GZIP 解压
- **Acceptance Criteria Addressed**: AC-1, AC-2
- **Test Requirements**:
  - `programmatic` TR-2.1: 解析标准 HTTP 请求/响应
  - `programmatic` TR-2.2: 解析分块编码响应
  - `programmatic` TR-2.3: 解压 GZIP 编码响应体
- **Notes**: 参考 RFC 7230 规范

## [x] Task 3: 实现 TLS ClientHello 解析器
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - 解析 TLS 1.2/1.3 ClientHello 消息
  - 提取 SNI（Server Name Indication）字段
  - 提取 TLS 版本和密码套件信息
- **Acceptance Criteria Addressed**: AC-2
- **Test Requirements**:
  - `programmatic` TR-3.1: 解析标准 TLS 1.2 ClientHello，提取 SNI
  - `programmatic` TR-3.2: 解析 TLS 1.3 ClientHello，提取 SNI
- **Notes**: 需要手动解析 TLS 握手协议

## [x] Task 4: 实现证书生成与管理服务
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - 生成根 CA 证书
  - 为指定域名动态生成叶子证书
  - 实现证书缓存机制
  - 实现 CA 证书安装到 Windows 受信任存储
- **Acceptance Criteria Addressed**: AC-2
- **Test Requirements**:
  - `programmatic` TR-4.1: 成功生成根 CA 证书
  - `programmatic` TR-4.2: 为 example.com 生成有效叶子证书
  - `human-judgment` TR-4.3: 根 CA 正确安装到系统证书存储
- **Notes**: 使用 System.Security.Cryptography.X509Certificates

## [x] Task 5: 实现 WFP 重定向服务
- **Priority**: P0
- **Depends On**: None
- **Description**: 
  - 封装 WFP API P/Invoke 调用
  - 注册 ALE 连接层过滤器
  - 将 TCP 80/443 流量重定向到本地代理端口
  - 实现过滤器清理机制
- **Acceptance Criteria Addressed**: AC-1, AC-2
- **Test Requirements**:
  - `programmatic` TR-5.1: 成功注册 WFP 过滤器
  - `programmatic` TR-5.2: 成功重定向 HTTP 流量到本地端口
- **Notes**: 需要管理员权限，参考 fwpuclnt.dll API

## [x] Task 6: 实现透明代理服务
- **Priority**: P0
- **Depends On**: Task 2, Task 3, Task 4, Task 5
- **Description**: 
  - 实现本地 TCP 监听服务（端口 9080/9443）
  - 实现 HTTP 透明代理（处理直接 HTTP 请求）
  - 实现 HTTPS MITM 代理（解析 SNI，生成证书，双向中继）
  - 记录流量到 HttpTrafficRecord
- **Acceptance Criteria Addressed**: AC-1, AC-2
- **Test Requirements**:
  - `programmatic` TR-6.1: 成功捕获并解析 HTTP 请求
  - `programmatic` TR-6.2: 成功解密 HTTPS 请求，显示明文响应体
- **Notes**: 使用 SslStream 实现 TLS 握手

## [x] Task 7: 实现流量列表表格 UI
- **Priority**: P0
- **Depends On**: Task 1
- **Description**: 
  - 在 MainWindow 中添加 DataGrid 控件
  - 绑定 ObservableCollection<HttpTrafficRecord>
  - 配置列：时间戳、方法、URL、状态码、响应时间、请求大小、响应大小
  - 添加启动/停止/清除按钮
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `human-judgment` TR-7.1: 表格显示所有要求的列
  - `human-judgment` TR-7.2: 按钮功能正常（启动/停止/清除）
- **Notes**: 使用 Avalonia DataGrid 的编译绑定

## [x] Task 8: 实现详情面板 UI
- **Priority**: P1
- **Depends On**: Task 1, Task 7
- **Description**: 
  - 添加详情面板到主窗口
  - 实现 TabControl：Headers、Body、Raw、Timing
  - 添加语法高亮（JSON/XML/HTML）
  - 实现十六进制查看器
- **Acceptance Criteria Addressed**: AC-7
- **Test Requirements**:
  - `human-judgment` TR-8.1: 详情面板显示完整的请求/响应信息
  - `human-judgment` TR-8.2: 语法高亮正确显示
- **Notes**: 使用 Avalonia 的 TabControl 和 TextBox

## [x] Task 9: 实现排序功能
- **Priority**: P1
- **Depends On**: Task 1, Task 7
- **Description**: 
  - 实现 DataGrid 列排序
  - 添加排序指示图标
  - 支持升序/降序切换
- **Acceptance Criteria Addressed**: AC-4
- **Test Requirements**:
  - `human-judgment` TR-9.1: 点击列标题可排序
  - `human-judgment` TR-9.2: 排序指示图标正确显示
- **Notes**: 使用 ICollectionView 实现排序

## [x] Task 10: 实现搜索功能
- **Priority**: P1
- **Depends On**: Task 1, Task 7
- **Description**: 
  - 添加搜索框到工具栏
  - 实现全局搜索（匹配 URL、请求体、响应体）
  - 高亮匹配结果
- **Acceptance Criteria Addressed**: AC-5
- **Test Requirements**:
  - `programmatic` TR-10.1: 搜索关键词能正确过滤记录
  - `human-judgment` TR-10.2: 匹配结果高亮显示
- **Notes**: 使用 ICollectionView.Filter

## [x] Task 11: 实现筛选功能
- **Priority**: P1
- **Depends On**: Task 1, Task 7
- **Description**: 
  - 添加筛选控件（请求方法、状态码范围）
  - 实现按方法筛选（GET/POST/PUT/DELETE 等）
  - 实现按状态码筛选（1xx/2xx/3xx/4xx/5xx）
- **Acceptance Criteria Addressed**: AC-6
- **Test Requirements**:
  - `programmatic` TR-11.1: 按方法筛选正确工作
  - `programmatic` TR-11.2: 按状态码范围筛选正确工作
- **Notes**: 结合搜索和筛选使用复合过滤器

## [x] Task 12: 实现 HAR 导出功能
- **Priority**: P1
- **Depends On**: Task 1
- **Description**: 
  - 实现 HAR 1.2 格式导出
  - 支持导出全部或选中记录
  - 保存为 JSON 文件
- **Acceptance Criteria Addressed**: AC-8
- **Test Requirements**:
  - `programmatic` TR-12.1: 生成的 HAR 文件符合 HAR 1.2 规范
  - `programmatic` TR-12.2: HAR 文件可被 Chrome DevTools 导入
- **Notes**: 参考 HAR 1.2 规范

## [x] Task 13: 性能优化与测试
- **Priority**: P2
- **Depends On**: Task 6
- **Description**: 
  - 实现流量记录滑动窗口（限制最大记录数）
  - 优化内存使用
  - 测试性能指标（CPU、内存）
- **Acceptance Criteria Addressed**: AC-9
- **Test Requirements**:
  - `programmatic` TR-13.1: 10000 条记录时内存 < 200 MB
  - `programmatic` TR-13.2: 持续捕获时 CPU < 20%
- **Notes**: 使用 BenchmarkDotNet 进行性能测试

## [x] Task 14: 集成测试与调试
- **Priority**: P2
- **Depends On**: All
- **Description**: 
  - 端到端集成测试
  - 修复发现的 bug
  - 优化用户体验
- **Acceptance Criteria Addressed**: All
- **Test Requirements**:
  - `human-judgment` TR-14.1: 整体功能正常工作
  - `human-judgment` TR-14.2: UI 响应流畅
- **Notes**: 需要手动测试各种场景