# Tasks

## 第一优先级：运行时关键缺陷修复
- [x] Task 1: 修复DataGrid数据绑定问题——FilteredConversations未绑定到UI
  - [x] SubTask 1.1: 检查MainWindow.axaml中DataGrid的ItemsSource绑定，确认绑定到正确的集合
  - [x] SubTask 1.2: 验证抓包后Conversations和FilteredConversations集合有数据
  - [x] SubTask 1.3: 运行应用验证DataGrid显示HTTP会话
- [x] Task 2: 修复控制台中文乱码问题
  - [x] SubTask 2.1: 在Program.cs中设置Console.OutputEncoding = Encoding.UTF8
  - [x] SubTask 2.2: 在Serilog控制台sink中配置UTF-8输出
  - [x] SubTask 2.3: 验证日志中中文适配器名称正确显示
- [x] Task 3: 修复详情面板数据不更新问题
  - [x] SubTask 3.1: 检查SelectedConversation绑定是否正确触发UpdateDetailPanel
  - [x] SubTask 3.2: 确认HttpConversation属性变更通知正确触发
  - [x] SubTask 3.3: 验证点击会话后详情面板显示内容

## 第二优先级：代码质量修复
- [x] Task 4: 修复CS0067未使用事件警告
  - [x] SubTask 4.1: 在WindowsPacketCaptureService中移除或实现PacketCaptured/StatisticsUpdated事件的触发
  - [x] SubTask 4.2: 在ProtocolParser中移除或实现ConversationCreated/ConversationUpdated事件的触发
  - [x] SubTask 4.3: 在CertificateManager中移除或实现CertificateInstalled/CertificateRemoved事件的触发
- [x] Task 5: 修复CA1416平台兼容性警告
  - [x] SubTask 5.1: 为WindowsIdentity/WindowsPrincipal添加[SupportedOSPlatform("windows")]守卫
  - [x] SubTask 5.2: 为X509Store操作添加平台守卫
  - [x] SubTask 5.3: 为Registry操作添加平台守卫
  - [x] SubTask 5.4: 验证编译警告数量减少
- [x] Task 6: 修复Nullable引用类型警告
  - [x] SubTask 6.1: 审查所有public API的nullable标注
  - [x] SubTask 6.2: 修复潜在的null引用异常（NetworkAdapterManager.cs CS8601）
  - [x] SubTask 6.3: 验证编译无nullable警告（仅剩NU1903第三方包警告）

## 第三优先级：缺失功能补全
- [x] Task 7: 实现DataGrid错误/慢请求行高亮
  - [x] SubTask 7.1: 为HttpConversation添加IsError和IsSlow属性通知
  - [x] SubTask 7.2: 在MainWindow.axaml中为DataGrid行添加条件样式（红色=错误，黄色=慢）
  - [x] SubTask 7.3: 验证4xx/5xx行红色高亮，慢请求黄色高亮
- [x] Task 8: 实现搜索结果高亮
  - [x] SubTask 8.1: 在MainWindowViewModel中添加搜索命令和搜索结果集合
  - [x] SubTask 8.2: 在UI工具栏添加搜索输入框和搜索按钮
  - [x] SubTask 8.3: 搜索匹配的会话在DataGrid中高亮显示
- [x] Task 9: 实现基础内容查看器增强
  - [x] SubTask 9.1: JSON响应自动格式化和语法着色
  - [x] SubTask 9.2: XML响应自动格式化
  - [x] SubTask 9.3: Hex查看器（二进制数据显示为十六进制+ASCII）

## 第四优先级：核心单元测试
- [x] Task 10: 创建测试项目并添加核心模块单元测试
  - [x] SubTask 10.1: 创建FlowReveal.Tests xUnit测试项目
  - [x] SubTask 10.2: 编写IpPacketParser单元测试（正常包、截断包、非IPv4包）
  - [x] SubTask 10.3: 编写HttpParser单元测试（GET请求、POST请求、Chunked响应、gZip响应）
  - [x] SubTask 10.4: 编写FilterEngine单元测试（各运算符、AND/OR组合、正则匹配）
  - [x] SubTask 10.5: 编写TcpReassembler单元测试（有序重组、乱序重组、FIN/RST处理）
  - [x] SubTask 10.6: 执行dotnet test验证所有测试通过（30/30通过）

## 第五优先级：安全与稳定性
- [x] Task 11: 安全审查与修复
  - [x] SubTask 11.1: 审查CertificateManager中证书密码硬编码问题
  - [x] SubTask 11.2: 审查HttpsProxyServer中SSL协议版本配置
  - [x] SubTask 11.3: 审查系统代理恢复机制可靠性（应用崩溃时能否恢复）
  - [x] SubTask 11.4: 修复发现的安全问题
- [x] Task 12: 稳定性改进
  - [x] SubTask 12.1: 添加全局异常处理（AppDomain.UnhandledException + TaskScheduler.UnobservedTaskException）
  - [x] SubTask 12.2: 确保应用退出时正确恢复系统代理设置
  - [x] SubTask 12.3: 添加内存使用监控和大流量自动清理机制

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 3] depends on [Task 1]
- [Task 4] depends on [Task 1]
- [Task 7] depends on [Task 1]
- [Task 8] depends on [Task 1]
- [Task 10] depends on [Task 4, Task 5, Task 6]
- [Task 11] depends on [Task 1]
- [Task 12] depends on [Task 11]
