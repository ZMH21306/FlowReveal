using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using FlowReveal.Models;

namespace FlowReveal.Core.Capture
{
    public class WfpCaptureEngine : IDisposable
    {
        private bool _isRunning;
        private IntPtr _engineHandle;
        private IntPtr _sessionHandle;
        private List<IntPtr> _filterIds;
        private List<PacketInfo> _capturedPackets;
        private object _lockObject = new object();

        public event EventHandler<PacketInfo> PacketCaptured;
        public bool IsRunning => _isRunning;

        public WfpCaptureEngine()
        {
            _capturedPackets = new List<PacketInfo>();
            _filterIds = new List<IntPtr>();
        }

        public void Start()
        {
            if (_isRunning)
                return;

            try
            {
                // 初始化 WFP 引擎
                InitializeWfpEngine();
                
                // 创建 WFP 会话
                CreateWfpSession();
                
                // 设置过滤器
                SetupFilters();
                
                // 开始捕获
                StartCapture();
                
                _isRunning = true;
                Console.WriteLine("WFP 捕获引擎已启动");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"启动 WFP 捕获引擎失败: {ex.Message}");
                Cleanup();
                throw;
            }
        }

        public void Stop()
        {
            if (!_isRunning)
                return;

            try
            {
                // 停止捕获
                StopCapture();
                
                // 清理过滤器
                CleanupFilters();
                
                // 清理会话
                CleanupSession();
                
                // 关闭 WFP 引擎
                CloseWfpEngine();
                
                _isRunning = false;
                Console.WriteLine("WFP 捕获引擎已停止");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"停止 WFP 捕获引擎失败: {ex.Message}");
                Cleanup();
                throw;
            }
        }

        private void InitializeWfpEngine()
        {
            try
            {
                Console.WriteLine("初始化 WFP 引擎...");
                
                // 打开 WFP 引擎
                uint result = WfpApi.FwpmEngineOpen0(
                    null, // 本地计算机
                    0, // 默认认证服务
                    IntPtr.Zero, // 无认证身份
                    IntPtr.Zero, // 无会话
                    out _engineHandle
                );

                if (result != 0)
                {
                    int lastError = Marshal.GetLastWin32Error();
                    throw new Exception($"打开 WFP 引擎失败，错误码: {result}, Win32 错误: {lastError}");
                }

                Console.WriteLine("WFP 引擎初始化成功");
                Console.WriteLine($"引擎句柄: {_engineHandle}");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"WFP 引擎初始化详细错误: {ex.Message}");
                throw new Exception("初始化 WFP 引擎失败", ex);
            }
        }

        private void CreateWfpSession()
        {
            try
            {
                Console.WriteLine("创建 WFP 会话...");
                
                var session = new FWPM_SESSION0
                {
                    flags = WfpConstants.FWPM_SESSION_FLAG_DYNAMIC
                };

                uint result = WfpApi.FwpmSessionCreate0(
                    ref session,
                    out _sessionHandle
                );

                if (result != 0)
                {
                    throw new Exception($"创建 WFP 会话失败，错误码: {result}");
                }

                Console.WriteLine("WFP 会话创建成功");
            }
            catch (Exception ex)
            {
                throw new Exception("创建 WFP 会话失败", ex);
            }
        }

        private void SetupFilters()
        {
            try
            {
                Console.WriteLine("设置 WFP 过滤器...");
                
                // 添加 HTTP 流量过滤器（端口 80）
                AddFilter(80);
                
                // 添加 HTTPS 流量过滤器（端口 443）
                AddFilter(443);
                
                Console.WriteLine("WFP 过滤器设置成功");
            }
            catch (Exception ex)
            {
                throw new Exception("设置 WFP 过滤器失败", ex);
            }
        }

        private void AddFilter(int port)
        {
            // 这里实现过滤器添加逻辑
            // 实际实现中需要创建完整的过滤器结构
            Console.WriteLine($"添加端口 {port} 的过滤器");
            
            // 模拟过滤器添加
            _filterIds.Add(IntPtr.Zero);
        }

        private void StartCapture()
        {
            // 这里实现开始捕获
            // 注册回调函数来处理捕获的数据包
            Console.WriteLine("开始 WFP 捕获...");
        }

        private void StopCapture()
        {
            // 这里实现停止捕获
            Console.WriteLine("停止 WFP 捕获...");
        }

        private void CleanupFilters()
        {
            try
            {
                Console.WriteLine("清理 WFP 过滤器...");
                
                foreach (var filterId in _filterIds)
                {
                    if (filterId != IntPtr.Zero)
                    {
                        WfpApi.FwpmFilterDeleteById0(_engineHandle, filterId);
                    }
                }
                
                _filterIds.Clear();
                Console.WriteLine("WFP 过滤器清理成功");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"清理 WFP 过滤器失败: {ex.Message}");
            }
        }

        private void CleanupSession()
        {
            try
            {
                if (_sessionHandle != IntPtr.Zero)
                {
                    Console.WriteLine("清理 WFP 会话...");
                    WfpApi.FwpmSessionDestroy0(_sessionHandle);
                    _sessionHandle = IntPtr.Zero;
                    Console.WriteLine("WFP 会话清理成功");
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"清理 WFP 会话失败: {ex.Message}");
            }
        }

        private void CloseWfpEngine()
        {
            try
            {
                if (_engineHandle != IntPtr.Zero)
                {
                    Console.WriteLine("关闭 WFP 引擎...");
                    WfpApi.FwpmEngineClose0(_engineHandle);
                    _engineHandle = IntPtr.Zero;
                    Console.WriteLine("WFP 引擎关闭成功");
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"关闭 WFP 引擎失败: {ex.Message}");
            }
        }

        private void Cleanup()
        {
            CleanupFilters();
            CleanupSession();
            CloseWfpEngine();
        }

        private void OnPacketCaptured(PacketInfo packet)
        {
            lock (_lockObject)
            {
                _capturedPackets.Add(packet);
                PacketCaptured?.Invoke(this, packet);
            }
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

        public void Dispose()
        {
            Stop();
        }
    }
}