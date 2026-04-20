using System;
using System.Threading;
using System.Threading.Tasks;
using FlowReveal.Core.Capture;
using FlowReveal.Core.Output;
using FlowReveal.Core.Parser;
using FlowReveal.Core.Session;
using FlowReveal.Models;

namespace FlowReveal.Services
{
    public class CaptureService : IDisposable
    {
        private PcapCaptureEngine _captureEngine;
        private PacketParser _packetParser;
        private TcpStreamAssembler _streamAssembler;
        private ConsolePacketWriter _consoleWriter;
        private CancellationTokenSource _cts;
        private Task _processingTask;

        public event EventHandler<PacketInfo> PacketCaptured;
        public event EventHandler<HttpMessage> HttpMessageCaptured;

        public bool IsRunning => _captureEngine?.IsRunning ?? false;

        public CaptureService()
        {
            _captureEngine = new PcapCaptureEngine();
            _packetParser = new PacketParser();
            _streamAssembler = new TcpStreamAssembler();
            _consoleWriter = new ConsolePacketWriter();
            _cts = new CancellationTokenSource();

            // 注册事件处理
            _captureEngine.PacketCaptured += OnPacketCaptured;
        }

        public void Start()
        {
            if (IsRunning)
                return;

            try
            {
                Console.WriteLine("启动网络捕获服务...");
                _captureEngine.Start();
                
                // 启动处理任务
                _processingTask = Task.Run(() => ProcessPackets(_cts.Token), _cts.Token);
                
                Console.WriteLine("网络捕获服务已启动");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"启动捕获服务失败: {ex.Message}");
                throw;
            }
        }

        public void Stop()
        {
            if (!IsRunning)
                return;

            try
            {
                Console.WriteLine("停止网络捕获服务...");
                
                // 取消处理任务
                _cts.Cancel();
                
                // 停止捕获引擎
                _captureEngine.Stop();
                
                // 等待处理任务完成
                if (_processingTask != null)
                {
                    _processingTask.Wait(5000);
                }
                
                Console.WriteLine("网络捕获服务已停止");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"停止捕获服务失败: {ex.Message}");
                throw;
            }
        }

        private void OnPacketCaptured(object sender, PacketInfo packet)
        {
            try
            {
                // 解析数据包
                var parsedPacket = _packetParser.ParseRawPacket(packet.Data, packet.Timestamp);
                if (parsedPacket != null)
                {
                    // 添加到流重组器
                    _streamAssembler.AddPacket(parsedPacket);
                    
                    // 输出到终端
                    _consoleWriter.WritePacket(parsedPacket);
                    
                    // 触发事件
                    PacketCaptured?.Invoke(this, parsedPacket);

                    // 检查是否有 HTTP 消息
                    var httpMessages = _streamAssembler.GetHttpMessages();
                    foreach (var message in httpMessages)
                    {
                        _consoleWriter.WriteHttpMessage(message);
                        HttpMessageCaptured?.Invoke(this, message);
                    }
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"处理数据包失败: {ex.Message}");
            }
        }

        private async Task ProcessPackets(CancellationToken cancellationToken)
        {
            try
            {
                while (!cancellationToken.IsCancellationRequested)
                {
                    // 处理队列中的数据包
                    await Task.Delay(100, cancellationToken);
                }
            }
            catch (TaskCanceledException)
            {
                // 正常取消
            }
            catch (Exception ex)
            {
                Console.WriteLine($"处理数据包任务失败: {ex.Message}");
            }
        }

        public void Dispose()
        {
            Stop();
            _cts.Dispose();
            _captureEngine.Dispose();
        }
    }
}