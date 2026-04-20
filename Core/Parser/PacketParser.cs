using System;
using FlowReveal.Models;

namespace FlowReveal.Core.Parser
{
    public class PacketParser
    {
        private const int ETHERNET_HEADER_LENGTH = 14;
        private const int IP_MIN_HEADER_LENGTH = 20;

        public PacketInfo ParseRawPacket(byte[] rawData, DateTime timestamp)
        {
            try
            {
                if (rawData == null || rawData.Length < ETHERNET_HEADER_LENGTH + IP_MIN_HEADER_LENGTH)
                {
                    return null;
                }

                // 检查以太网帧类型 (0x0800 = IPv4)
                if (rawData[12] != 0x08 || rawData[13] != 0x00)
                {
                    return null;
                }

                // 解析以太网头部后的 IP 数据包
                int ipOffset = ETHERNET_HEADER_LENGTH;

                // 解析 IP 头部
                var ipHeader = new IpHeader();
                if (!ipHeader.Parse(rawData, ipOffset))
                {
                    return null;
                }

                // 计算 IP 头部的实际长度
                int ipHeaderLength = ipHeader.HeaderLength * 4;

                // 解析传输层协议
                var packetInfo = new PacketInfo
                {
                    Timestamp = timestamp,
                    SourceIp = ipHeader.SourceIp,
                    DestinationIp = ipHeader.DestinationIp,
                    PacketSize = rawData.Length,
                    Data = rawData
                };

                // 根据协议类型解析
                switch (ipHeader.Protocol)
                {
                    case 6: // TCP
                        packetInfo.Protocol = ProtocolType.TCP;
                        ParseTcpPacket(rawData, ipOffset + ipHeaderLength, packetInfo);
                        break;
                    case 17: // UDP
                        packetInfo.Protocol = ProtocolType.UDP;
                        ParseUdpPacket(rawData, ipOffset + ipHeaderLength, packetInfo);
                        break;
                    default:
                        packetInfo.Protocol = ProtocolType.Other;
                        break;
                }

                return packetInfo;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"解析数据包失败: {ex.Message}");
                return null;
            }
        }

        private void ParseTcpPacket(byte[] data, int tcpOffset, PacketInfo packetInfo)
        {
            if (data.Length < tcpOffset + 20)
                return;

            var tcpHeader = new TcpHeader();
            if (tcpHeader.Parse(data, tcpOffset))
            {
                packetInfo.SourcePort = tcpHeader.SourcePort;
                packetInfo.DestinationPort = tcpHeader.DestinationPort;
            }
        }

        private void ParseUdpPacket(byte[] data, int udpOffset, PacketInfo packetInfo)
        {
            if (data.Length < udpOffset + 8)
                return;

            var udpHeader = new UdpHeader();
            if (udpHeader.Parse(data, udpOffset))
            {
                packetInfo.SourcePort = udpHeader.SourcePort;
                packetInfo.DestinationPort = udpHeader.DestinationPort;
            }
        }
    }
}