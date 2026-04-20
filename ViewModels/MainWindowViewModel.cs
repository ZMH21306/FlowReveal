using System;
using System.Collections.ObjectModel;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Input;
using Avalonia.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using FlowReveal.Core.Capture;
using FlowReveal.Models;
using FlowReveal.Services;

namespace FlowReveal.ViewModels
{
    public partial class MainWindowViewModel : ObservableObject, IDisposable
    {
        private CaptureService? _captureService;
        private CancellationTokenSource? _cts;

        [ObservableProperty]
        private ObservableCollection<PacketInfo> _packets = new();

        [ObservableProperty]
        private PacketInfo? _selectedPacket;

        [ObservableProperty]
        private string _statusText = "已停止";

        [ObservableProperty]
        private string _statusMessage = "准备就绪";

        [ObservableProperty]
        private string _startButtonText = "开始捕获";

        [ObservableProperty]
        private int _packetCount;

        [ObservableProperty]
        private string _dataPreview = "";

        private bool _isCapturing;

        public MainWindowViewModel()
        {
            _packetCount = 0;
            _isCapturing = false;
        }

        partial void OnSelectedPacketChanged(PacketInfo? value)
        {
            if (value != null)
            {
                DataPreview = GenerateDataPreview(value);
            }
            else
            {
                DataPreview = "";
            }
        }

        [RelayCommand]
        private void ToggleCapture()
        {
            if (_isCapturing)
            {
                StopCapture();
            }
            else
            {
                StartCapture();
            }
        }

        [RelayCommand]
        private void Clear()
        {
            Packets.Clear();
            PacketCount = 0;
            SelectedPacket = null;
            StatusMessage = "已清除捕获数据";
        }

        private void StartCapture()
        {
            try
            {
                _captureService = new CaptureService();
                _cts = new CancellationTokenSource();

                _captureService.PacketCaptured += OnPacketCaptured;
                _captureService.HttpMessageCaptured += OnHttpMessageCaptured;

                _captureService.Start();

                _isCapturing = true;
                StatusText = "正在捕获...";
                StartButtonText = "停止捕获";
                StatusMessage = "正在捕获网络流量...";

                Task.Run(() => MonitorCapture(_cts.Token));
            }
            catch (Exception ex)
            {
                StatusMessage = $"启动失败: {ex.Message}";
                StatusText = "错误";
            }
        }

        private void StopCapture()
        {
            try
            {
                _cts?.Cancel();
                _captureService?.Stop();
                _captureService?.Dispose();

                _isCapturing = false;
                StatusText = "已停止";
                StartButtonText = "开始捕获";
                StatusMessage = $"捕获已停止，共捕获 {PacketCount} 个数据包";
            }
            catch (Exception ex)
            {
                StatusMessage = $"停止失败: {ex.Message}";
            }
        }

        private async Task MonitorCapture(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested && _isCapturing)
            {
                await Task.Delay(1000, cancellationToken);
            }
        }

        private void OnPacketCaptured(object? sender, PacketInfo packet)
        {
            if (packet == null) return;

            try
            {
                Dispatcher.UIThread.Post(() =>
                {
                    Packets.Add(packet);
                    PacketCount = Packets.Count;

                    if (PacketCount % 100 == 0)
                    {
                        StatusMessage = $"已捕获 {PacketCount} 个数据包...";
                    }
                });
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"处理数据包失败: {ex.Message}");
            }
        }

        private void OnHttpMessageCaptured(object? sender, HttpMessage message)
        {
            if (message == null) return;

            try
            {
                Dispatcher.UIThread.Post(() =>
                {
                    StatusMessage = message.IsRequest
                        ? $"HTTP 请求: {message.Method} {message.Url}"
                        : $"HTTP 响应: {message.StatusCode} {message.StatusMessage}";
                });
            }
            catch (Exception ex)
            {
                System.Diagnostics.Debug.WriteLine($"处理 HTTP 消息失败: {ex.Message}");
            }
        }

        private string GenerateDataPreview(PacketInfo packet)
        {
            if (packet.Data == null || packet.Data.Length == 0)
                return "无数据";

            try
            {
                var sb = new StringBuilder();
                int dataStart = 14; // 以太网头部

                // 解析 IP 头部长度
                if (packet.Data.Length >= 14 + 1)
                {
                    byte ipHeaderLength = (byte)((packet.Data[14] & 0x0F) * 4);
                    dataStart += ipHeaderLength;

                    // 解析传输层头部长度
                    if (packet.Protocol == ProtocolType.TCP && packet.Data.Length >= dataStart + 12)
                    {
                        byte tcpHeaderLength = (byte)((packet.Data[dataStart + 12] >> 4) * 4);
                        dataStart += tcpHeaderLength;
                    }
                    else if (packet.Protocol == ProtocolType.UDP && packet.Data.Length >= dataStart + 8)
                    {
                        dataStart += 8;
                    }
                }

                // 显示数据预览
                if (dataStart < packet.Data.Length)
                {
                    int dataLength = packet.Data.Length - dataStart;
                    int previewLength = Math.Min(dataLength, 256);
                    byte[] data = new byte[previewLength];
                    Array.Copy(packet.Data, dataStart, data, 0, previewLength);

                    for (int i = 0; i < previewLength; i += 16)
                    {
                        int lineLength = Math.Min(16, previewLength - i);
                        StringBuilder hexLine = new StringBuilder();
                        StringBuilder asciiLine = new StringBuilder();

                        for (int j = 0; j < lineLength; j++)
                        {
                            byte b = data[i + j];
                            hexLine.Append($"{b:X2} ");
                            asciiLine.Append((b >= 32 && b <= 126) ? (char)b : '.');
                        }

                        hexLine.Append(new string(' ', (16 - lineLength) * 3));
                        sb.AppendLine($"{hexLine}  {asciiLine}");
                    }

                    if (dataLength > previewLength)
                    {
                        sb.AppendLine($"... (truncated, total {dataLength} bytes)");
                    }
                }
                else
                {
                    sb.AppendLine("No payload");
                }

                return sb.ToString();
            }
            catch (Exception ex)
            {
                return $"Error: {ex.Message}";
            }
        }

        public void Dispose()
        {
            StopCapture();
            _cts?.Dispose();
        }
    }
}