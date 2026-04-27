#pragma warning disable CS0067
using System;
using System.Collections.Generic;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using FlowReveal.Platforms.Windows.Network;
using FlowReveal.Platforms.Windows.Security;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Platforms.Windows.Capture
{
    public class WindowsPacketCaptureService : IPacketCaptureService
    {
        private readonly ILogger<WindowsPacketCaptureService> _logger;
        private readonly PrivilegeManager _privilegeManager;
        private readonly NetworkAdapterManager _adapterManager;

        private Socket? _captureSocket;
        private CancellationTokenSource? _cts;
        private Task? _captureTask;
        private bool _isCapturing;
        private CaptureEngineType _engineType = CaptureEngineType.RawSocket;
        private NetworkAdapter? _currentAdapter;
        private long _droppedPackets;

        private readonly CaptureStatistics _statistics = new();
        private readonly List<NetworkAdapter> _adapters = new();
        private readonly object _lock = new();

        public event EventHandler<RawPacket>? PacketCaptured;
        public event EventHandler<CaptureStatistics>? StatisticsUpdated;
        public event EventHandler<string>? StatusChanged;

        public bool IsCapturing => _isCapturing;
        public CaptureEngineType EngineType => _engineType;
        public CaptureStatistics Statistics => _statistics;
        public IReadOnlyList<NetworkAdapter> AvailableAdapters
        {
            get
            {
                lock (_lock)
                {
                    return _adapters.AsReadOnly();
                }
            }
        }

        public WindowsPacketCaptureService(
            ILogger<WindowsPacketCaptureService> logger,
            PrivilegeManager privilegeManager,
            NetworkAdapterManager adapterManager)
        {
            _logger = logger;
            _privilegeManager = privilegeManager;
            _adapterManager = adapterManager;

            _logger.LogInformation("WindowsPacketCaptureService initialized with RawSocket engine");

            RefreshAdapterList();
        }

        public async Task StartCaptureAsync(NetworkAdapter adapter, CancellationToken cancellationToken = default)
        {
            if (_isCapturing)
            {
                _logger.LogWarning("捕获已在进行中，先停止");
                await StopCaptureAsync();
            }

            if (!_privilegeManager.IsRunningAsAdmin)
            {
                _logger.LogCritical("无法在没有管理员权限的情况下启动捕获");
                throw new UnauthorizedAccessException("需要管理员权限才能进行数据包捕获");
            }

            _currentAdapter = adapter;
            _logger.LogInformation("开始原始套接字捕获，适配器: {AdapterName} (索引: {Index})",
                adapter.FriendlyName, adapter.Index);

            try
            {
                _captureSocket = new Socket(AddressFamily.InterNetwork, SocketType.Raw, ProtocolType.IP);
                _captureSocket.SetSocketOption(SocketOptionLevel.IP, SocketOptionName.HeaderIncluded, true);

                var bindAddress = adapter.IpAddresses.Count > 0
                    ? adapter.IpAddresses[0]
                    : IPAddress.Any;

                _captureSocket.Bind(new IPEndPoint(bindAddress, 0));

                _captureSocket.IOControl(IOControlCode.ReceiveAll, new byte[] { 1, 0, 0, 0 }, null);

                _captureSocket.ReceiveBufferSize = 1024 * 1024;

                _cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);

                _statistics.CaptureStartTime = DateTime.UtcNow;
                _statistics.CaptureEndTime = DateTime.MinValue;
                _statistics.TotalPacketsCaptured = 0;
                _statistics.TotalPacketsDropped = 0;
                _statistics.TotalBytesCaptured = 0;
                _statistics.TotalHttpConversations = 0;
                _statistics.ActiveTcpSessions = 0;
                _droppedPackets = 0;

                _isCapturing = true;
                StatusChanged?.Invoke(this, $"正在捕获: {adapter.FriendlyName}");

                _captureTask = Task.Run(() => CaptureLoop(_cts.Token), _cts.Token);

                _logger.LogInformation("原始套接字捕获已成功在 {Address} 启动", bindAddress);
            }
            catch (Exception ex)
            {
                _isCapturing = false;
                _logger.LogError(ex, "启动原始套接字捕获失败");
                StatusChanged?.Invoke(this, "捕获启动失败");
                throw;
            }

            await Task.CompletedTask;
        }

        public async Task StopCaptureAsync()
        {
            if (!_isCapturing)
            {
                _logger.LogDebug("捕获未在进行中，无需停止");
                return;
            }

            _logger.LogInformation("停止原始套接字捕获...");

            _isCapturing = false;
            _cts?.Cancel();

            try
            {
                _captureSocket?.Shutdown(SocketShutdown.Both);
                _captureSocket?.Close();
                _captureSocket = null;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "关闭捕获套接字时出错");
            }

            if (_captureTask != null)
            {
                try
                {
                    await _captureTask;
                }
                catch (OperationCanceledException)
                {
                    _logger.LogDebug("捕获任务已按预期取消");
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "等待捕获任务完成时出错");
                }
                _captureTask = null;
            }

            _statistics.CaptureEndTime = DateTime.UtcNow;
            _cts?.Dispose();
            _cts = null;

            StatusChanged?.Invoke(this, "已停止");
            StatisticsUpdated?.Invoke(this, _statistics);

            _logger.LogInformation("捕获已停止。总数据包: {TotalPackets}, 丢弃: {DroppedPackets}, 字节: {TotalBytes}",
                _statistics.TotalPacketsCaptured, _statistics.TotalPacketsDropped, _statistics.TotalBytesCaptured);
        }

        private void CaptureLoop(CancellationToken cancellationToken)
        {
            _logger.LogInformation("捕获循环已启动");

            var buffer = new byte[65535];
            var lastStatsTime = DateTime.UtcNow;

            try
            {
                while (!cancellationToken.IsCancellationRequested)
                {
                    try
                    {
                        var bytesRead = _captureSocket?.Receive(buffer) ?? 0;
                        if (bytesRead > 0)
                        {
                            ProcessReceivedData(buffer, bytesRead);
                        }
                    }
                    catch (SocketException ex) when (ex.SocketErrorCode == SocketError.Interrupted)
                    {
                        _logger.LogDebug("套接字已中断，退出捕获循环");
                        break;
                    }
                    catch (SocketException ex) when (ex.SocketErrorCode == SocketError.ConnectionReset)
                    {
                        _logger.LogDebug("套接字连接已重置，退出捕获循环");
                        break;
                    }
                    catch (SocketException ex) when (ex.SocketErrorCode == SocketError.OperationAborted)
                    {
                        _logger.LogDebug("套接字操作已中止，退出捕获循环");
                        break;
                    }
                    catch (SocketException ex)
                    {
                        _logger.LogWarning(ex, "捕获循环中的套接字错误");
                        if (cancellationToken.IsCancellationRequested) break;
                        Thread.Sleep(10);
                    }
                    catch (ObjectDisposedException)
                    {
                        _logger.LogDebug("套接字已释放，退出捕获循环");
                        break;
                    }

                    lock (_lock)
                    {
                        var now = DateTime.UtcNow;
                        var elapsed = (now - lastStatsTime).TotalSeconds;
                        if (elapsed >= 1.0)
                        {
                            _statistics.InstantPacketsPerSecond = _statistics.TotalPacketsCaptured / elapsed;
                            lastStatsTime = now;
                            StatisticsUpdated?.Invoke(this, _statistics);
                        }
                    }
                }
            }
            catch (OperationCanceledException)
            {
                _logger.LogDebug("捕获循环已取消");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "捕获循环中出现意外错误");
            }
            finally
            {
                _logger.LogInformation("捕获循环已结束");
            }
        }

        private void ProcessReceivedData(byte[] buffer, int bytesRead)
        {
            try
            {
                if (!IpPacketParser.TryParse(buffer, 0, bytesRead, out var parsed))
                {
                    _statistics.TotalPacketsDropped++;
                    return;
                }

                var packet = new RawPacket
                {
                    Data = new byte[bytesRead],
                    Timestamp = DateTime.UtcNow,
                    Length = bytesRead,
                    SourceIp = parsed.SourceIp,
                    DestinationIp = parsed.DestinationIp,
                    SourcePort = parsed.SourcePort,
                    DestinationPort = parsed.DestinationPort,
                    Protocol = parsed.Protocol,
                    NetworkInterface = _currentAdapter?.FriendlyName ?? "未知"
                };

                Array.Copy(buffer, packet.Data, bytesRead);

                lock (_lock)
                {
                    _statistics.TotalPacketsCaptured++;
                    _statistics.TotalBytesCaptured += bytesRead;
                }

                PacketCaptured?.Invoke(this, packet);
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "处理接收到的数据包时出错");
                Interlocked.Increment(ref _droppedPackets);
            }
        }

        private void RefreshAdapterList()
        {
            var adapters = _adapterManager.RefreshAdapters();
            lock (_lock)
            {
                _adapters.Clear();
                _adapters.AddRange(adapters);
            }
            _logger.LogInformation("适配器列表已刷新: 可用适配器 {Count} 个", _adapters.Count);
        }

        public void Dispose()
        {
            if (_isCapturing)
            {
                try
                {
                    StopCaptureAsync().GetAwaiter().GetResult();
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "释放期间停止捕获时出错");
                }
            }

            _captureSocket?.Dispose();
            _cts?.Dispose();
            _logger.LogInformation("WindowsPacketCaptureService 已释放");
        }
    }
}
