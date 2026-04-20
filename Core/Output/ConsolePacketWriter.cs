using System;
using System.Text;
using FlowReveal.Models;

namespace FlowReveal.Core.Output
{
    public class ConsolePacketWriter
    {
        public void WritePacket(PacketInfo packet)
        {
            if (packet == null)
                return;

            try
            {
                var sb = new StringBuilder();
                
                // 时间戳和基本信息
                sb.AppendLine($"┌───────────────────────────────────────────────────────────────────────────────────────");
                sb.AppendLine($"│ [ {packet.Timestamp:yyyy-MM-dd HH:mm:ss.fff} ] [ {packet.Protocol} ]");
                sb.AppendLine($"│ Source:      {packet.SourceIp}:{packet.SourcePort}");
                sb.AppendLine($"│ Destination: {packet.DestinationIp}:{packet.DestinationPort}");
                sb.AppendLine($"│ Size:        {packet.PacketSize} bytes");
                
                // 应用信息
                if (!string.IsNullOrEmpty(packet.ApplicationInfo))
                {
                    sb.AppendLine($"│ Application: {packet.ApplicationInfo}");
                }

                // 协议详细信息
                if (packet.Protocol == ProtocolType.TCP)
                {
                    WriteTcpDetails(sb, packet);
                }
                else if (packet.Protocol == ProtocolType.UDP)
                {
                    WriteUdpDetails(sb, packet);
                }

                // 数据预览
                WriteDataPreview(sb, packet);

                sb.AppendLine($"└───────────────────────────────────────────────────────────────────────────────────────");
                sb.AppendLine();
                Console.WriteLine(sb.ToString());
            }
            catch (Exception ex)
            {
                Console.WriteLine($"输出数据包信息失败: {ex.Message}");
            }
        }

        public void WriteHttpMessage(HttpMessage message)
        {
            if (message == null)
                return;

            try
            {
                var sb = new StringBuilder();
                
                sb.AppendLine($"┌───────────────────────────────────────────────────────────────────────────────────────");
                if (message.IsRequest)
                {
                    sb.AppendLine($"│ [ {DateTime.Now:yyyy-MM-dd HH:mm:ss.fff} ] [ HTTP REQUEST ]");
                    sb.AppendLine($"│ {message.Method} {message.Url} {message.HttpVersion}");
                }
                else
                {
                    sb.AppendLine($"│ [ {DateTime.Now:yyyy-MM-dd HH:mm:ss.fff} ] [ HTTP RESPONSE ]");
                    sb.AppendLine($"│ {message.HttpVersion} {message.StatusCode} {message.StatusMessage}");
                }

                // 头部信息
                sb.AppendLine($"│");
                sb.AppendLine($"│ Headers:");
                foreach (var header in message.Headers)
                {
                    sb.AppendLine($"│   {header.Key}: {header.Value}");
                }

                // Cookie 信息
                if (message.Cookies != null && message.Cookies.Count > 0)
                {
                    sb.AppendLine($"│");
                    sb.AppendLine($"│ Cookies:");
                    foreach (var cookie in message.Cookies)
                    {
                        sb.AppendLine($"│   {cookie.Key}: {cookie.Value}");
                    }
                }

                // 查询参数
                if (message.QueryParameters != null && message.QueryParameters.Count > 0)
                {
                    sb.AppendLine($"│");
                    sb.AppendLine($"│ Query Parameters:");
                    foreach (var param in message.QueryParameters)
                    {
                        sb.AppendLine($"│   {param.Key}: {param.Value}");
                    }
                }

                // 响应体预览
                if (!string.IsNullOrEmpty(message.Body))
                {
                    sb.AppendLine($"│");
                    sb.AppendLine($"│ Body ({message.BodySize} bytes):");
                    string preview = message.Body.Length > 500 ? message.Body.Substring(0, 500) + "..." : message.Body;
                    foreach (var line in preview.Split('\n'))
                    {
                        sb.AppendLine($"│   {line}");
                    }
                }

                sb.AppendLine($"└───────────────────────────────────────────────────────────────────────────────────────");
                sb.AppendLine();
                Console.WriteLine(sb.ToString());
            }
            catch (Exception ex)
            {
                Console.WriteLine($"输出 HTTP 消息失败: {ex.Message}");
            }
        }

        private void WriteTcpDetails(StringBuilder sb, PacketInfo packet)
        {
            try
            {
                if (packet.Data != null && packet.Data.Length >= 54) // 以太网(14) + IP(20) + TCP(20)
                {
                    // 解析 TCP 头部
                    int tcpOffset = 34; // 14 + 20
                    byte[] tcpHeader = new byte[20];
                    Array.Copy(packet.Data, tcpOffset, tcpHeader, 0, 20);

                    // 源端口和目标端口
                    ushort sourcePort = BitConverter.ToUInt16(tcpHeader, 0);
                    ushort destPort = BitConverter.ToUInt16(tcpHeader, 2);

                    // 序列号和确认号
                    uint sequence = BitConverter.ToUInt32(tcpHeader, 4);
                    uint ack = BitConverter.ToUInt32(tcpHeader, 8);

                    // 头部长度
                    byte dataOffset = (byte)((tcpHeader[12] >> 4) * 4);

                    // 标志
                    byte flags = tcpHeader[13];
                    bool syn = (flags & 0x02) != 0;
                    bool ackFlag = (flags & 0x10) != 0;
                    bool fin = (flags & 0x01) != 0;
                    bool rst = (flags & 0x04) != 0;
                    bool psh = (flags & 0x08) != 0;
                    bool urg = (flags & 0x20) != 0;

                    // 窗口大小
                    ushort window = BitConverter.ToUInt16(tcpHeader, 14);

                    sb.AppendLine($"│");
                    sb.AppendLine($"│ TCP Details:");
                    sb.AppendLine($"│   Source Port:      {sourcePort}");
                    sb.AppendLine($"│   Destination Port: {destPort}");
                    sb.AppendLine($"│   Sequence Number:  {sequence}");
                    sb.AppendLine($"│   Acknowledgment:   {ack}");
                    sb.AppendLine($"│   Header Length:    {dataOffset} bytes");
                    sb.AppendLine($"│   Flags:            {GetTcpFlagsString(syn, ackFlag, fin, rst, psh, urg)}");
                    sb.AppendLine($"│   Window Size:      {window}");
                }
            }
            catch
            {
                // 解析失败时忽略
            }
        }

        private void WriteUdpDetails(StringBuilder sb, PacketInfo packet)
        {
            try
            {
                if (packet.Data != null && packet.Data.Length >= 42) // 以太网(14) + IP(20) + UDP(8)
                {
                    // 解析 UDP 头部
                    int udpOffset = 34; // 14 + 20
                    byte[] udpHeader = new byte[8];
                    Array.Copy(packet.Data, udpOffset, udpHeader, 0, 8);

                    // 源端口和目标端口
                    ushort sourcePort = BitConverter.ToUInt16(udpHeader, 0);
                    ushort destPort = BitConverter.ToUInt16(udpHeader, 2);

                    // 长度
                    ushort length = BitConverter.ToUInt16(udpHeader, 4);

                    sb.AppendLine($"│");
                    sb.AppendLine($"│ UDP Details:");
                    sb.AppendLine($"│   Source Port:      {sourcePort}");
                    sb.AppendLine($"│   Destination Port: {destPort}");
                    sb.AppendLine($"│   Length:           {length} bytes");
                }
            }
            catch
            {
                // 解析失败时忽略
            }
        }

        private void WriteDataPreview(StringBuilder sb, PacketInfo packet)
        {
            try
            {
                if (packet.Data != null && packet.Data.Length > 0)
                {
                    // 计算有效数据开始位置
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
                            dataStart += 8; // UDP 头部固定 8 字节
                        }
                    }

                    // 显示数据预览
                    if (dataStart < packet.Data.Length)
                    {
                        int dataLength = packet.Data.Length - dataStart;
                        int previewLength = Math.Min(dataLength, 128);
                        byte[] data = new byte[previewLength];
                        Array.Copy(packet.Data, dataStart, data, 0, previewLength);

                        sb.AppendLine($"│");
                        sb.AppendLine($"│ Data Preview ({dataLength} bytes):");
                        
                        // 十六进制和 ASCII 预览
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

                            // 补齐空格
                            hexLine.Append(new string(' ', (16 - lineLength) * 3));
                            sb.AppendLine($"│   {hexLine.ToString()}  {asciiLine.ToString()}");
                        }

                        if (dataLength > previewLength)
                        {
                            sb.AppendLine($"│   ... (truncated, total {dataLength} bytes)");
                        }
                    }
                    else
                    {
                        sb.AppendLine($"│");
                        sb.AppendLine($"│ Data: No payload");
                    }
                }
            }
            catch
            {
                // 解析失败时忽略
            }
        }

        private string GetTcpFlagsString(bool syn, bool ack, bool fin, bool rst, bool psh, bool urg)
        {
            var flags = new System.Collections.Generic.List<string>();
            if (syn) flags.Add("SYN");
            if (ack) flags.Add("ACK");
            if (fin) flags.Add("FIN");
            if (rst) flags.Add("RST");
            if (psh) flags.Add("PSH");
            if (urg) flags.Add("URG");
            return flags.Count > 0 ? string.Join(", ", flags) : "None";
        }
    }
}