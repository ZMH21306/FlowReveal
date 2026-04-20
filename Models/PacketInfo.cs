using System;

namespace FlowReveal.Models
{
    public class PacketInfo
    {
        public DateTime Timestamp { get; set; }
        public string SourceIp { get; set; }
        public int SourcePort { get; set; }
        public string DestinationIp { get; set; }
        public int DestinationPort { get; set; }
        public ProtocolType Protocol { get; set; }
        public int PacketSize { get; set; }
        public byte[] Data { get; set; }
        public string Direction { get; set; }
        public string ApplicationInfo { get; set; }
    }

    public enum ProtocolType
    {
        TCP,
        UDP,
        ICMP,
        Other
    }
}