# Tasks

- [ ] Task 1: 修复DataGrid数据绑定——绑定到FilteredConversations
  - [ ] SubTask 1.1: 修改MainWindow.axaml中DataGrid的ItemsSource绑定到FilteredConversations
  - [ ] SubTask 1.2: 验证过滤功能正常工作
- [ ] Task 2: 实现错误会话自动高亮
  - [ ] SubTask 2.1: 在MainWindow.axaml中为DataGrid行添加条件样式（4xx/5xx红色，慢请求黄色）
  - [ ] SubTask 2.2: 验证高亮显示正常
- [ ] Task 3: 实现HAR格式导出
  - [ ] SubTask 3.1: 在ISessionStore接口中添加ExportHarAsync方法
  - [ ] SubTask 3.2: 在SessionStore中实现HAR 1.2格式导出
- [ ] Task 4: 实现请求编辑重发功能
  - [ ] SubTask 4.1: 创建RequestEditorViewModel（编辑URL、方法、头部、正文）
  - [ ] SubTask 4.2: 创建RequestEditorDialog.axaml弹窗
  - [ ] SubTask 4.3: 在MainWindowViewModel中添加ResendCommand
  - [ ] SubTask 4.4: 实现HTTP请求发送和响应捕获
- [ ] Task 5: 修复HttpsProxyServer中继数据解析
  - [ ] SubTask 5.1: 修改RelayDataAsync，在数据中继时解析HTTP请求/响应
  - [ ] SubTask 5.2: 通过ConversationCaptured事件发出完整的HttpConversation
- [ ] Task 6: 修复ProtocolParser中TCP流数据消费问题
  - [ ] SubTask 6.1: 确保OnSessionDataReceived中数据消费后正确移除已处理字节
  - [ ] SubTask 6.2: 添加流数据缓冲区溢出保护
- [ ] Task 7: 添加核心模块单元测试项目
  - [ ] SubTask 7.1: 创建FlowReveal.Tests测试项目
  - [ ] SubTask 7.2: 编写IpPacketParser单元测试
  - [ ] SubTask 7.3: 编写HttpParser单元测试（含边界条件）
  - [ ] SubTask 7.4: 编写FilterEngine单元测试
  - [ ] SubTask 7.5: 编写TcpReassembler单元测试（含乱序场景）
  - [ ] SubTask 7.6: 运行所有测试验证通过

# Task Dependencies
- [Task 2] depends on [Task 1]
- [Task 4] depends on [Task 1]
- [Task 5] depends on [Task 6]
- [Task 7] depends on [Task 6]
