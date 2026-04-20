using System;
using System.Text;

namespace FlowReveal.Core.Parser
{
    public class IpHeader
    {
        public byte Version { get; set; }
        public byte HeaderLength { get; set; }
        public byte TOS { get; set; }
        public ushort TotalLength { get; set; }
        public ushort Identification { get; set; }
        public ushort FlagsAndOffset { get; set; }
        public byte TTL { get; set; }
        public byte Protocol { get; set; }
        public ushort Checksum { get; set; }
        public string SourceIp { get; set; }
        public string DestinationIp { get; set; }

        public bool Parse(byte[] data, int offset)
        {
            try
            {
                if (data.Length < offset + 20)
                    return false;

                Version = (byte)((data[offset] >> 4) & 0x0F);
                HeaderLength = (byte)(data[offset] & 0x0F);
                TOS = data[offset + 1];
                TotalLength = BitConverter.ToUInt16(data, offset + 2);
                Identification = BitConverter.ToUInt16(data, offset + 4);
                FlagsAndOffset = BitConverter.ToUInt16(data, offset + 6);
                TTL = data[offset + 8];
                Protocol = data[offset + 9];
                Checksum = BitConverter.ToUInt16(data, offset + 10);

                // 解析源 IP 地址
                var sourceIpBytes = new byte[4];
                Array.Copy(data, offset + 12, sourceIpBytes, 0, 4);
                SourceIp = $"{sourceIpBytes[0]}.{sourceIpBytes[1]}.{sourceIpBytes[2]}.{sourceIpBytes[3]}";

                // 解析目标 IP 地址
                var destIpBytes = new byte[4];
                Array.Copy(data, offset + 16, destIpBytes, 0, 4);
                DestinationIp = $"{destIpBytes[0]}.{destIpBytes[1]}.{destIpBytes[2]}.{destIpBytes[3]}";

                return true;
            }
            catch
            {
                return false;
            }
        }
    }
}