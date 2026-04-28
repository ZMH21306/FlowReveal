using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public interface ICaptureService
    {
        Task StartCaptureAsync();
        Task StopCaptureAsync();
        event EventHandler<CaptureDataEventArgs> DataReceived;
    }

    public class CaptureDataEventArgs : EventArgs
    {
        public uint ProcessId { get; set; }
        public uint RemotePort { get; set; }
        public uint LocalPort { get; set; }
        public byte Protocol { get; set; }
        public byte[] Data { get; set; }
        public DateTime Timestamp { get; set; }
    }

    public class CaptureService : ICaptureService
    {
        private const string DevicePath = @"\\.\FlowReveal";
        private IntPtr _deviceHandle = IntPtr.Zero;
        private Thread _readThread;
        private bool _isRunning;

        public event EventHandler<CaptureDataEventArgs> DataReceived;

        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        private static extern IntPtr CreateFile(
            string fileName,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool DeviceIoControl(
            IntPtr hDevice,
            uint dwIoControlCode,
            IntPtr lpInBuffer,
            uint nInBufferSize,
            IntPtr lpOutBuffer,
            uint nOutBufferSize,
            out uint lpBytesReturned,
            IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool CloseHandle(IntPtr hObject);

        private const uint GENERIC_READ = 0x80000000;
        private const uint GENERIC_WRITE = 0x40000000;
        private const uint FILE_SHARE_READ = 0x00000001;
        private const uint FILE_SHARE_WRITE = 0x00000002;
        private const uint OPEN_EXISTING = 3;
        private const uint FILE_ATTRIBUTE_NORMAL = 0x80;

        private const uint IOCTL_FLOWREVEAL_START = 0x80002000;
        private const uint IOCTL_FLOWREVEAL_STOP = 0x80002004;
        private const uint IOCTL_FLOWREVEAL_GET_PACKET = 0x80002008;

        public async Task StartCaptureAsync()
        {
            if (_isRunning)
                return;

            await Task.Run(() =>
            {
                // 打开设备
                _deviceHandle = CreateFile(
                    DevicePath,
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    IntPtr.Zero,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    IntPtr.Zero);

                if (_deviceHandle == (IntPtr)(-1))
                {
                    throw new IOException("Failed to open device. Driver may not be installed.", Marshal.GetLastWin32Error());
                }

                // 发送启动命令
                uint bytesReturned;
                bool success = DeviceIoControl(
                    _deviceHandle,
                    IOCTL_FLOWREVEAL_START,
                    IntPtr.Zero,
                    0,
                    IntPtr.Zero,
                    0,
                    out bytesReturned,
                    IntPtr.Zero);

                if (!success)
                {
                    CloseHandle(_deviceHandle);
                    _deviceHandle = IntPtr.Zero;
                    throw new IOException("Failed to start capture.", Marshal.GetLastWin32Error());
                }

                _isRunning = true;

                // 启动读取线程
                _readThread = new Thread(ReadLoop);
                _readThread.IsBackground = true;
                _readThread.Start();
            });
        }

        public async Task StopCaptureAsync()
        {
            if (!_isRunning)
                return;

            await Task.Run(() =>
            {
                _isRunning = false;

                // 发送停止命令
                if (_deviceHandle != IntPtr.Zero)
                {
                    uint bytesReturned;
                    DeviceIoControl(
                        _deviceHandle,
                        IOCTL_FLOWREVEAL_STOP,
                        IntPtr.Zero,
                        0,
                        IntPtr.Zero,
                        0,
                        out bytesReturned,
                        IntPtr.Zero);

                    CloseHandle(_deviceHandle);
                    _deviceHandle = IntPtr.Zero;
                }

                // 等待线程结束
                _readThread?.Join();
            });
        }

        private void ReadLoop()
        {
            while (_isRunning && _deviceHandle != IntPtr.Zero)
            {
                try
                {
                    // 分配缓冲区
                    int bufferSize = 65536;
                    IntPtr buffer = Marshal.AllocHGlobal(bufferSize);

                    try
                    {
                        uint bytesReturned;
                        bool success = DeviceIoControl(
                            _deviceHandle,
                            IOCTL_FLOWREVEAL_GET_PACKET,
                            IntPtr.Zero,
                            0,
                            buffer,
                            (uint)bufferSize,
                            out bytesReturned,
                            IntPtr.Zero);

                        if (success && bytesReturned > 0)
                        {
                            // 解析数据包信息
                            var args = ParsePacket(buffer, bytesReturned);
                            DataReceived?.Invoke(this, args);
                        }
                        else
                        {
                            // 没有数据，短暂等待
                            Thread.Sleep(10);
                        }
                    }
                    finally
                    {
                        Marshal.FreeHGlobal(buffer);
                    }
                }
                catch (Exception ex)
                {
                    // 记录错误，继续运行
                    Console.WriteLine($"Capture error: {ex.Message}");
                    Thread.Sleep(100);
                }
            }
        }

        private CaptureDataEventArgs ParsePacket(IntPtr buffer, uint length)
        {
            // 解析数据包结构
            // 结构: timestamp(8) + processId(4) + remotePort(4) + localPort(4) + protocol(1) + ipVersion(1) + reserved(2) + dataLength(4) + data

            uint offset = 0;

            // 时间戳
            long timestamp = Marshal.ReadInt64(buffer, (int)offset);
            offset += 8;

            // 进程 ID
            uint processId = (uint)Marshal.ReadInt32(buffer, (int)offset);
            offset += 4;

            // 远程端口
            uint remotePort = (uint)Marshal.ReadInt32(buffer, (int)offset);
            offset += 4;

            // 本地端口
            uint localPort = (uint)Marshal.ReadInt32(buffer, (int)offset);
            offset += 4;

            // 协议
            byte protocol = Marshal.ReadByte(buffer, (int)offset);
            offset += 1;

            // IP 版本
            offset += 1; // 跳过 ipVersion

            // 保留字节
            offset += 2;

            // 数据长度
            uint dataLength = (uint)Marshal.ReadInt32(buffer, (int)offset);
            offset += 4;

            // 数据
            byte[] data = new byte[dataLength];
            Marshal.Copy(buffer + (int)offset, data, 0, (int)dataLength);

            return new CaptureDataEventArgs
            {
                Timestamp = DateTime.FromFileTime(timestamp),
                ProcessId = processId,
                RemotePort = remotePort,
                LocalPort = localPort,
                Protocol = protocol,
                Data = data
            };
        }
    }
}
