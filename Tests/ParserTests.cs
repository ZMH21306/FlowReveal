using System;
using FlowReveal.Core.Parser;
using FlowReveal.Models;

namespace FlowReveal.Tests
{
    public class ParserTests
    {
        public static void TestIpParser()
        {
            Console.WriteLine("测试 IP 解析器...");
            
            // 模拟 IP 数据包
            byte[] ipPacket = new byte[] {
                0x45, 0x00, 0x00, 0x28, 0x12, 0x34, 0x40, 0x00,
                0x40, 0x06, 0x00, 0x00, 0xC0, 0xA8, 0x01, 0x01,
                0x08, 0x08, 0x08, 0x08
            };

            var ipHeader = new IpHeader();
            bool success = ipHeader.Parse(ipPacket, 0);
            
            if (success)
            {
                Console.WriteLine($"IP 版本: {ipHeader.Version}");
                Console.WriteLine($"头部长度: {ipHeader.HeaderLength}");
                Console.WriteLine($"源 IP: {ipHeader.SourceIp}");
                Console.WriteLine($"目标 IP: {ipHeader.DestinationIp}");
                Console.WriteLine($"协议: {ipHeader.Protocol}");
                Console.WriteLine("IP 解析器测试成功!");
            }
            else
            {
                Console.WriteLine("IP 解析器测试失败!");
            }
        }

        public static void TestTcpParser()
        {
            Console.WriteLine("\n测试 TCP 解析器...");
            
            // 模拟 TCP 头部
            byte[] tcpPacket = new byte[] {
                0x13, 0x88, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01,
                0x00, 0x00, 0x00, 0x00, 0x50, 0x02, 0x72, 0x10,
                0x00, 0x00, 0x00, 0x00
            };

            var tcpHeader = new TcpHeader();
            bool success = tcpHeader.Parse(tcpPacket, 0);
            
            if (success)
            {
                Console.WriteLine($"源端口: {tcpHeader.SourcePort}");
                Console.WriteLine($"目标端口: {tcpHeader.DestinationPort}");
                Console.WriteLine($"序列号: {tcpHeader.SequenceNumber}");
                Console.WriteLine($"确认号: {tcpHeader.AcknowledgmentNumber}");
                Console.WriteLine($"头部长度: {tcpHeader.HeaderLength}");
                Console.WriteLine($"SYN: {tcpHeader.IsSyn}");
                Console.WriteLine($"ACK: {tcpHeader.IsAck}");
                Console.WriteLine("TCP 解析器测试成功!");
            }
            else
            {
                Console.WriteLine("TCP 解析器测试失败!");
            }
        }

        public static void TestHttpParser()
        {
            Console.WriteLine("\n测试 HTTP 解析器...");
            
            // 模拟 HTTP 请求
            string httpRequest = "GET /index.html HTTP/1.1\r\n" +
                               "Host: www.example.com\r\n" +
                               "User-Agent: Mozilla/5.0\r\n" +
                               "Accept: text/html\r\n" +
                               "\r\n";

            byte[] httpData = System.Text.Encoding.UTF8.GetBytes(httpRequest);
            var httpParser = new HttpParser();
            var request = httpParser.ParseHttpRequest(httpData, 0, httpData.Length);
            
            if (request != null)
            {
                Console.WriteLine($"方法: {request.Method}");
                Console.WriteLine($"URL: {request.Url}");
                Console.WriteLine($"HTTP 版本: {request.HttpVersion}");
                Console.WriteLine($"Host: {request.Host}");
                Console.WriteLine($"User-Agent: {request.UserAgent}");
                Console.WriteLine("HTTP 请求解析测试成功!");
            }
            else
            {
                Console.WriteLine("HTTP 请求解析测试失败!");
            }

            // 模拟 HTTP 响应
            string httpResponse = "HTTP/1.1 200 OK\r\n" +
                                "Content-Type: text/html\r\n" +
                                "Content-Length: 12\r\n" +
                                "\r\n" +
                                "Hello World!";

            byte[] responseData = System.Text.Encoding.UTF8.GetBytes(httpResponse);
            var response = httpParser.ParseHttpResponse(responseData, 0, responseData.Length);
            
            if (response != null)
            {
                Console.WriteLine($"状态码: {response.StatusCode}");
                Console.WriteLine($"状态消息: {response.StatusMessage}");
                Console.WriteLine($"Content-Type: {response.ContentType}");
                Console.WriteLine($"响应体: {response.Body}");
                Console.WriteLine("HTTP 响应解析测试成功!");
            }
            else
            {
                Console.WriteLine("HTTP 响应解析测试失败!");
            }
        }

        public static void TestPacketParser()
        {
            Console.WriteLine("\n测试数据包解析器...");
            
            // 模拟完整的 IP + TCP + HTTP 数据包
            byte[] packetData = new byte[] {
                // IP 头部
                0x45, 0x00, 0x00, 0x3C, 0x12, 0x34, 0x40, 0x00,
                0x40, 0x06, 0x00, 0x00, 0xC0, 0xA8, 0x01, 0x01,
                0x08, 0x08, 0x08, 0x08,
                // TCP 头部
                0x13, 0x88, 0x00, 0x50, 0x00, 0x00, 0x00, 0x01,
                0x00, 0x00, 0x00, 0x00, 0x50, 0x02, 0x72, 0x10,
                0x00, 0x00, 0x00, 0x00,
                // HTTP 请求
                0x47, 0x45, 0x54, 0x20, 0x2F, 0x20, 0x48, 0x54,
                0x54, 0x50, 0x2F, 0x31, 0x2E, 0x31, 0x0D, 0x0A,
                0x48, 0x6F, 0x73, 0x74, 0x3A, 0x20, 0x77, 0x77,
                0x77, 0x2E, 0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C,
                0x65, 0x2E, 0x63, 0x6F, 0x6D, 0x0D, 0x0A, 0x0D,
                0x0A
            };

            var packetParser = new PacketParser();
            var packet = packetParser.ParseRawPacket(packetData, DateTime.Now);
            
            if (packet != null)
            {
                Console.WriteLine($"协议: {packet.Protocol}");
                Console.WriteLine($"源 IP: {packet.SourceIp}");
                Console.WriteLine($"源端口: {packet.SourcePort}");
                Console.WriteLine($"目标 IP: {packet.DestinationIp}");
                Console.WriteLine($"目标端口: {packet.DestinationPort}");
                Console.WriteLine($"数据包大小: {packet.PacketSize}");
                Console.WriteLine("数据包解析器测试成功!");
            }
            else
            {
                Console.WriteLine("数据包解析器测试失败!");
            }
        }
    }
}