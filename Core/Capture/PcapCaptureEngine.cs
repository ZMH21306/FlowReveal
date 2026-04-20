using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using FlowReveal.Models;
using FlowReveal.Core.Parser;
using FlowReveal.Core.Output;
using SharpPcap;
using SharpPcap.LibPcap;

namespace FlowReveal.Core.Capture
{
    public class PcapCaptureEngine : IDisposable
    {
        private bool _isRunning;
        private List<ILiveDevice> _devices;
        private PacketParser _packetParser;
        private ConsolePacketWriter _consoleWriter;
        private List<PacketInfo> _capturedPackets;
        private object _lockObject = new object();

        public event EventHandler<PacketInfo>? PacketCaptured;
        public bool IsRunning => _isRunning;

        public PcapCaptureEngine()
        {
            _capturedPackets = new List<PacketInfo>();
            _packetParser = new PacketParser();
            _consoleWriter = new ConsolePacketWriter();
            _devices = new List<ILiveDevice>();
        }

        public void Start()
        {
            if (_isRunning)
                return;

            try
            {
                Console.WriteLine("初始化 Pcap 捕获引擎...");
                
                // 获取网络设备列表
                var devices = LibPcapLiveDeviceList.Instance;
                
                if (devices.Count == 0)
                {
                    throw new Exception("未找到网络设备");
                }

                Console.WriteLine($"找到 {devices.Count} 个网络设备");
                
                // 列出所有设备
                foreach (var device in devices)
                {
                    Console.WriteLine($"  - {device.Name}: {device.Description}");
                }

                // 为每个设备启动捕获
                foreach (var device in devices)
                {
                    // 打开设备
                    device.Open(DeviceModes.Promiscuous, 1000);
                    Console.WriteLine($"开始捕获设备: {device.Name}");
                    
                    // 注册事件处理
                    device.OnPacketArrival += Device_OnPacketArrival;
                    
                    // 开始捕获
                    device.StartCapture();
                    _devices.Add(device);
                }

                _isRunning = true;
                Console.WriteLine("Pcap 捕获引擎已启动");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"启动 Pcap 捕获引擎失败: {ex.Message}");
                Cleanup();
                throw;
            }
        }

        private void Device_OnPacketArrival(object sender, PacketCapture e)
        {
            try
            {
                var rawPacket = e.Data.ToArray();
                var packetInfo = _packetParser.ParseRawPacket(rawPacket, DateTime.Now);
                
                if (packetInfo != null)
                {
                    OnPacketCaptured(packetInfo);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"处理数据包失败: {ex.Message}");
            }
        }

        public void Stop()
        {
            if (!_isRunning)
                return;

            try
            {
                Console.WriteLine("停止 Pcap 捕获引擎...");
                
                // 停止所有设备的捕获
                foreach (var device in _devices)
                {
                    try
                    {
                        device.StopCapture();
                        device.Close();
                        Console.WriteLine($"已停止捕获设备: {device.Name}");
                    }
                    catch
                    {
                        // 忽略关闭错误
                    }
                }
                _devices.Clear();
                
                _isRunning = false;
                Console.WriteLine("Pcap 捕获引擎已停止");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"停止 Pcap 捕获引擎失败: {ex.Message}");
                Cleanup();
                throw;
            }
        }

        private void OnPacketCaptured(PacketInfo packet)
        {
            if (packet == null)
                return;

            lock (_lockObject)
            {
                _capturedPackets.Add(packet);
                PacketCaptured?.Invoke(this, packet);
            }

            // 输出到终端
            _consoleWriter.WritePacket(packet);
        }

        public List<PacketInfo> GetCapturedPackets()
        {
            lock (_lockObject)
            {
                return new List<PacketInfo>(_capturedPackets);
            }
        }

        public void ClearCapturedPackets()
        {
            lock (_lockObject)
            {
                _capturedPackets.Clear();
            }
        }

        private void Cleanup()
        {
            Stop();
        }

        public void Dispose()
        {
            Cleanup();
        }
    }
}