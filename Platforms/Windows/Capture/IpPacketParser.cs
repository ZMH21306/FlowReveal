using System;
using System.Net;
using System.Runtime.InteropServices;

namespace FlowReveal.Platforms.Windows.Capture
{
    internal static class IpPacketParser
    {
        public static bool TryParse(byte[] buffer, int offset, int length, out ParsedIpPacket result)
        {
            result = default;

            if (length < offset + 20)
                return false;

            int version = (buffer[offset] >> 4) & 0x0F;
            if (version != 4)
                return false;

            int headerLength = (buffer[offset] & 0x0F) * 4;
            int totalLength = (buffer[offset + 2] << 8) | buffer[offset + 3];
            byte protocol = buffer[offset + 9];
            uint sourceIp = (uint)((buffer[offset + 12] << 24) | (buffer[offset + 13] << 16) | (buffer[offset + 14] << 8) | buffer[offset + 15]);
            uint destIp = (uint)((buffer[offset + 16] << 24) | (buffer[offset + 17] << 16) | (buffer[offset + 18] << 8) | buffer[offset + 19]);

            ushort sourcePort = 0;
            ushort destPort = 0;
            int payloadOffset = offset + headerLength;
            int payloadLength = totalLength - headerLength;

            if (protocol == 6 || protocol == 17)
            {
                if (length >= payloadOffset + 4)
                {
                    sourcePort = (ushort)((buffer[payloadOffset] << 8) | buffer[payloadOffset + 1]);
                    destPort = (ushort)((buffer[payloadOffset + 2] << 8) | buffer[payloadOffset + 3]);
                }
            }

            result = new ParsedIpPacket
            {
                Version = version,
                HeaderLength = headerLength,
                TotalLength = totalLength,
                Protocol = protocol,
                SourceIp = new IPAddress(sourceIp),
                DestinationIp = new IPAddress(destIp),
                SourcePort = sourcePort,
                DestinationPort = destPort,
                PayloadOffset = payloadOffset,
                PayloadLength = payloadLength
            };

            return true;
        }
    }

    internal struct ParsedIpPacket
    {
        public int Version;
        public int HeaderLength;
        public int TotalLength;
        public byte Protocol;
        public IPAddress SourceIp;
        public IPAddress DestinationIp;
        public ushort SourcePort;
        public ushort DestinationPort;
        public int PayloadOffset;
        public int PayloadLength;
    }
}
