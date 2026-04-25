using System;
using System.Net;

namespace FlowReveal.Core.Models
{
    public class RawPacket
    {
        public byte[] Data { get; set; } = Array.Empty<byte>();
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;
        public int Length { get; set; }
        public IPAddress SourceIp { get; set; } = IPAddress.None;
        public IPAddress DestinationIp { get; set; } = IPAddress.None;
        public ushort SourcePort { get; set; }
        public ushort DestinationPort { get; set; }
        public byte Protocol { get; set; }
        public string NetworkInterface { get; set; } = string.Empty;
    }
}
