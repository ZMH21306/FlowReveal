using System.Net;
using FlowReveal.Platforms.Windows.Capture;

namespace FlowReveal.Tests;

public class IpPacketParserTests
{
    private static byte[] BuildIpPacket(byte protocol = 6, ushort sourcePort = 12345, ushort destPort = 80, byte[]? payload = null, int headerLength = 20)
    {
        var totalLength = headerLength + (protocol == 6 || protocol == 17 ? 20 : 0) + (payload?.Length ?? 0);
        var packet = new byte[totalLength];

        packet[0] = (byte)((4 << 4) | (headerLength / 4));
        packet[1] = 0;
        packet[2] = (byte)((totalLength >> 8) & 0xFF);
        packet[3] = (byte)(totalLength & 0xFF);
        packet[4] = 0;
        packet[5] = 0;
        packet[6] = 0;
        packet[7] = 0;
        packet[8] = 64;
        packet[9] = protocol;

        packet[12] = 192;
        packet[13] = 168;
        packet[14] = 1;
        packet[15] = 1;

        packet[16] = 10;
        packet[17] = 0;
        packet[18] = 0;
        packet[19] = 1;

        if (protocol == 6 || protocol == 17)
        {
            int transportOffset = headerLength;
            packet[transportOffset] = (byte)((sourcePort >> 8) & 0xFF);
            packet[transportOffset + 1] = (byte)(sourcePort & 0xFF);
            packet[transportOffset + 2] = (byte)((destPort >> 8) & 0xFF);
            packet[transportOffset + 3] = (byte)(destPort & 0xFF);

            if (payload != null && payload.Length > 0)
            {
                Array.Copy(payload, 0, packet, transportOffset + 20, payload.Length);
            }
        }

        return packet;
    }

    [Fact]
    public void TryParse_ValidTcpPacket_ReturnsTrueWithCorrectFields()
    {
        var packet = BuildIpPacket(protocol: 6, sourcePort: 12345, destPort: 80);

        var result = IpPacketParser.TryParse(packet, 0, packet.Length, out var parsed);

        Assert.True(result);
        Assert.Equal(4, parsed.Version);
        Assert.Equal(20, parsed.HeaderLength);
        Assert.Equal(packet.Length, parsed.TotalLength);
        Assert.Equal(6, parsed.Protocol);
        Assert.Equal(12345, parsed.SourcePort);
        Assert.Equal(80, parsed.DestinationPort);
        Assert.Equal(new IPAddress(unchecked((uint)(192 << 24 | 168 << 16 | 1 << 8 | 1))), parsed.SourceIp);
        Assert.Equal(new IPAddress(unchecked((uint)(10 << 24 | 0 << 16 | 0 << 8 | 1))), parsed.DestinationIp);
    }

    [Fact]
    public void TryParse_ValidUdpPacket_ReturnsTrueWithCorrectFields()
    {
        var packet = BuildIpPacket(protocol: 17, sourcePort: 54321, destPort: 53);

        var result = IpPacketParser.TryParse(packet, 0, packet.Length, out var parsed);

        Assert.True(result);
        Assert.Equal(17, parsed.Protocol);
        Assert.Equal(54321, parsed.SourcePort);
        Assert.Equal(53, parsed.DestinationPort);
    }

    [Fact]
    public void TryParse_TruncatedPacket_ReturnsFalse()
    {
        var packet = new byte[10];

        var result = IpPacketParser.TryParse(packet, 0, packet.Length, out var parsed);

        Assert.False(result);
    }

    [Fact]
    public void TryParse_NonIpv4Packet_ReturnsFalse()
    {
        var packet = new byte[40];
        packet[0] = (byte)((6 << 4) | 5);

        var result = IpPacketParser.TryParse(packet, 0, packet.Length, out var parsed);

        Assert.False(result);
    }
}
