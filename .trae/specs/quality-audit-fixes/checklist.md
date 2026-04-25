# FlowReveal质量审查验收检查清单

## 数据绑定与UI修复
- [ ] DataGrid绑定到FilteredConversations，过滤功能正常
- [ ] 4xx/5xx状态码会话以红色文字显示
- [ ] 慢请求会话以黄色文字显示
- [ ] 过滤条件应用后DataGrid正确更新

## 数据导出
- [ ] HAR格式导出生成符合HAR 1.2规范的JSON文件
- [ ] JSON导出正常
- [ ] CSV导出正常
- [ ] PCAP导出正常

## 请求编辑重发
- [ ] 点击重发按钮打开编辑器弹窗
- [ ] 编辑器可修改URL、方法、头部和正文
- [ ] 发送请求后捕获并展示响应

## HTTPS代理修复
- [ ] CONNECT隧道中继数据正确解析为HttpConversation
- [ ] ConversationCaptured事件包含完整的请求/响应数据

## 协议解析修复
- [ ] TCP流数据消费后已处理字节正确移除
- [ ] 流数据缓冲区有溢出保护

## 单元测试
- [ ] FlowReveal.Tests项目创建成功
- [ ] IpPacketParser单元测试通过
- [ ] HttpParser单元测试通过（含边界条件）
- [ ] FilterEngine单元测试通过
- [ ] TcpReassembler单元测试通过（含乱序场景）
- [ ] 所有测试通过，核心模块覆盖率≥70%

## 编译质量
- [ ] dotnet build 0错误
- [ ] 无新增编译警告
