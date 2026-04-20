using System;
using System.Collections.Generic;
using System.Text;
using FlowReveal.Models;

namespace FlowReveal.Core.Session
{
    public class TcpStreamAssembler
    {
        private Dictionary<string, TcpStream> _streams;
        private object _lockObject = new object();
        private const int ETHERNET_HEADER_LENGTH = 14;
        private const int IP_MIN_HEADER_LENGTH = 20;
        private const int TCP_MIN_HEADER_LENGTH = 20;

        public TcpStreamAssembler()
        {
            _streams = new Dictionary<string, TcpStream>();
        }

        public void AddPacket(PacketInfo packet)
        {
            if (packet == null || packet.Protocol != ProtocolType.TCP)
                return;

            string streamKey = GetStreamKey(packet);
            
            lock (_lockObject)
            {
                if (!_streams.TryGetValue(streamKey, out var stream))
                {
                    stream = new TcpStream(packet);
                    _streams[streamKey] = stream;
                }

                stream.AddPacket(packet);

                // 清理已完成的流
                if (stream.IsComplete)
                {
                    _streams.Remove(streamKey);
                }
            }
        }

        public List<HttpMessage> GetHttpMessages()
        {
            var httpMessages = new List<HttpMessage>();
            var parser = new Core.Parser.HttpParser();

            lock (_lockObject)
            {
                foreach (var stream in _streams.Values)
                {
                    var data = stream.GetCompleteData();
                    if (data != null && data.Length > 0)
                    {
                        try
                        {
                            // 尝试解析 HTTP 消息
                            var message = parser.ParseHttpRequest(data, 0, data.Length);
                            if (message == null)
                            {
                                message = parser.ParseHttpResponse(data, 0, data.Length);
                            }
                            if (message != null)
                            {
                                httpMessages.Add(message);
                            }
                        }
                        catch
                        {
                            // 解析失败时忽略
                        }
                    }
                }
            }

            return httpMessages;
        }

        private string GetStreamKey(PacketInfo packet)
        {
            return $"{packet.SourceIp}:{packet.SourcePort}-{packet.DestinationIp}:{packet.DestinationPort}";
        }

        public void Clear()
        {
            lock (_lockObject)
            {
                _streams.Clear();
            }
        }
    }

    internal class TcpStream
    {
        private const int ETHERNET_HEADER_LENGTH = 14;
        private const int IP_MIN_HEADER_LENGTH = 20;
        private const int TCP_MIN_HEADER_LENGTH = 20;

        public string SourceIp { get; private set; }
        public int SourcePort { get; private set; }
        public string DestinationIp { get; private set; }
        public int DestinationPort { get; private set; }
        public bool IsComplete { get; private set; }

        private List<byte[]> _payloads;
        private uint _expectedSequence;
        private bool _hasSyn;
        private bool _hasFin;

        public TcpStream(PacketInfo firstPacket)
        {
            SourceIp = firstPacket.SourceIp;
            SourcePort = firstPacket.SourcePort;
            DestinationIp = firstPacket.DestinationIp;
            DestinationPort = firstPacket.DestinationPort;
            _payloads = new List<byte[]>();
            _expectedSequence = 0;
            _hasSyn = false;
            _hasFin = false;
        }

        public void AddPacket(PacketInfo packet)
        {
            // 提取 TCP 负载数据
            int headerSize = ETHERNET_HEADER_LENGTH + IP_MIN_HEADER_LENGTH + TCP_MIN_HEADER_LENGTH;
            if (packet.Data != null && packet.Data.Length > headerSize)
            {
                int payloadLength = packet.Data.Length - headerSize;
                byte[] payload = new byte[payloadLength];
                Array.Copy(packet.Data, headerSize, payload, 0, payloadLength);
                _payloads.Add(payload);
            }
            
            // 检查 TCP 标志
            if (packet.Data != null && packet.Data.Length >= ETHERNET_HEADER_LENGTH + IP_MIN_HEADER_LENGTH + TCP_MIN_HEADER_LENGTH)
            {
                int tcpOffset = ETHERNET_HEADER_LENGTH + IP_MIN_HEADER_LENGTH + 13; // TCP flags offset
                byte flags = packet.Data[tcpOffset];
                if ((flags & 0x02) != 0) // SYN
                    _hasSyn = true;
                if ((flags & 0x01) != 0) // FIN
                    _hasFin = true;
            }

            // 简化处理：认为有数据就可能是完整的
            IsComplete = _payloads.Count > 0;
        }

        public byte[] GetCompleteData()
        {
            if (_payloads.Count == 0)
                return null;

            int totalLength = 0;
            foreach (var payload in _payloads)
            {
                totalLength += payload.Length;
            }

            if (totalLength <= 0)
                return null;

            var result = new byte[totalLength];
            int offset = 0;
            foreach (var payload in _payloads)
            {
                Array.Copy(payload, 0, result, offset, payload.Length);
                offset += payload.Length;
            }

            return result;
        }
    }
}