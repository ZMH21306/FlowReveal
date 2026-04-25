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
                _logger.LogWarning("Capture already in progress, stopping first");
                await StopCaptureAsync();
            }

            if (!_privilegeManager.IsRunningAsAdmin)
            {
                _logger.LogCritical("Cannot start capture without Administrator privileges");
                throw new UnauthorizedAccessException("Administrator privileges required for packet capture");
            }

            _currentAdapter = adapter;
            _logger.LogInformation("Starting raw socket capture on adapter: {AdapterName} (Index: {Index})",
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
                StatusChanged?.Invoke(this, $"Capturing on {adapter.FriendlyName}");

                _captureTask = Task.Run(() => CaptureLoop(_cts.Token), _cts.Token);

                _logger.LogInformation("Raw socket capture started successfully on {Address}", bindAddress);
            }
            catch (Exception ex)
            {
                _isCapturing = false;
                _logger.LogError(ex, "Failed to start raw socket capture");
                StatusChanged?.Invoke(this, "Capture failed to start");
                throw;
            }

            await Task.CompletedTask;
        }

        public async Task StopCaptureAsync()
        {
            if (!_isCapturing)
            {
                _logger.LogDebug("Capture not in progress, nothing to stop");
                return;
            }

            _logger.LogInformation("Stopping raw socket capture...");

            _isCapturing = false;
            _cts?.Cancel();

            try
            {
                _captureSocket?.Close();
                _captureSocket = null;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Error closing capture socket");
            }

            if (_captureTask != null)
            {
                try
                {
                    await _captureTask;
                }
                catch (OperationCanceledException)
                {
                    _logger.LogDebug("Capture task cancelled as expected");
                }
                catch (Exception ex)
                {
                    _logger.LogWarning(ex, "Error waiting for capture task to complete");
                }
                _captureTask = null;
            }

            _statistics.CaptureEndTime = DateTime.UtcNow;
            _cts?.Dispose();
            _cts = null;

            StatusChanged?.Invoke(this, "Stopped");
            StatisticsUpdated?.Invoke(this, _statistics);

            _logger.LogInformation("Capture stopped. Total packets: {TotalPackets}, Dropped: {DroppedPackets}, Bytes: {TotalBytes}",
                _statistics.TotalPacketsCaptured, _statistics.TotalPacketsDropped, _statistics.TotalBytesCaptured);
        }

        private void CaptureLoop(CancellationToken cancellationToken)
        {
            _logger.LogInformation("Capture loop started");

            var buffer = new byte[65535];

            try
            {
                while (!cancellationToken.IsCancellationRequested)
                {
                    try
                    {
                        if (_captureSocket == null || (!_captureSocket.Connected && _captureSocket.Available == 0))
                        {
                            if (_captureSocket?.Available == 0)
                            {
                                var asyncResult = _captureSocket?.BeginReceive(buffer, 0, buffer.Length, SocketFlags.None, null, null);
                                if (asyncResult != null)
                                {
                                    var waitHandles = new[] { asyncResult.AsyncWaitHandle, cancellationToken.WaitHandle };
                                    var index = WaitHandle.WaitAny(waitHandles);

                                    if (index == 1)
                                    {
                                        _logger.LogDebug("Capture loop cancelled via cancellation token");
                                        break;
                                    }

                                    var bytesRead = _captureSocket?.EndReceive(asyncResult) ?? 0;
                                    if (bytesRead > 0)
                                    {
                                        ProcessReceivedData(buffer, bytesRead);
                                    }
                                }
                            }
                        }
                        else
                        {
                            if (_captureSocket.Available > 0)
                            {
                                var bytesRead = _captureSocket.Receive(buffer, 0, buffer.Length, SocketFlags.None);
                                if (bytesRead > 0)
                                {
                                    ProcessReceivedData(buffer, bytesRead);
                                }
                            }
                            else
                            {
                                Thread.Sleep(1);
                            }
                        }
                    }
                    catch (SocketException ex) when (ex.SocketErrorCode == SocketError.Interrupted)
                    {
                        _logger.LogDebug("Socket interrupted, exiting capture loop");
                        break;
                    }
                    catch (SocketException ex)
                    {
                        _logger.LogWarning(ex, "Socket error in capture loop");
                        if (cancellationToken.IsCancellationRequested) break;
                        Thread.Sleep(10);
                    }
                    catch (ObjectDisposedException)
                    {
                        _logger.LogDebug("Socket disposed, exiting capture loop");
                        break;
                    }
                }
            }
            catch (OperationCanceledException)
            {
                _logger.LogDebug("Capture loop cancelled");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Unexpected error in capture loop");
            }
            finally
            {
                _logger.LogInformation("Capture loop ended");
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
                    NetworkInterface = _currentAdapter?.FriendlyName ?? "Unknown"
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
                _logger.LogWarning(ex, "Error processing received packet");
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
            _logger.LogInformation("Adapter list refreshed: {Count} adapters available", _adapters.Count);
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
                    _logger.LogWarning(ex, "Error stopping capture during dispose");
                }
            }

            _captureSocket?.Dispose();
            _cts?.Dispose();
            _logger.LogInformation("WindowsPacketCaptureService disposed");
        }
    }
}
