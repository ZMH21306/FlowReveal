using System.Net;
using FlowReveal.Core.Models;
using FlowReveal.Services.Parser;
using Microsoft.Extensions.Logging.Abstractions;

namespace FlowReveal.Tests;

public class TcpReassemblerTests
{
    private readonly TcpReassembler _reassembler = new(NullLogger<TcpReassembler>.Instance);

    private static RawPacket BuildTcpPacket(
        IPAddress srcIp, ushort srcPort,
        IPAddress dstIp, ushort dstPort,
        uint seqNum, uint ackNum,
        bool syn = false, bool ack = false, bool fin = false, bool rst = false,
        byte[]? payload = null,
        int ipHeaderLength = 20,
        int tcpHeaderLength = 20)
    {
        int totalLength = ipHeaderLength + tcpHeaderLength + (payload?.Length ?? 0);
        var data = new byte[totalLength];

        data[0] = (byte)((4 << 4) | (ipHeaderLength / 4));

        int tcpOffset = ipHeaderLength;

        data[tcpOffset + 4] = (byte)((seqNum >> 24) & 0xFF);
        data[tcpOffset + 5] = (byte)((seqNum >> 16) & 0xFF);
        data[tcpOffset + 6] = (byte)((seqNum >> 8) & 0xFF);
        data[tcpOffset + 7] = (byte)(seqNum & 0xFF);

        data[tcpOffset + 8] = (byte)((ackNum >> 24) & 0xFF);
        data[tcpOffset + 9] = (byte)((ackNum >> 16) & 0xFF);
        data[tcpOffset + 10] = (byte)((ackNum >> 8) & 0xFF);
        data[tcpOffset + 11] = (byte)(ackNum & 0xFF);

        data[tcpOffset + 12] = (byte)((tcpHeaderLength / 4) << 4);

        byte flags = 0;
        if (fin) flags |= 0x01;
        if (syn) flags |= 0x02;
        if (rst) flags |= 0x04;
        if (ack) flags |= 0x10;
        data[tcpOffset + 13] = flags;

        if (payload != null && payload.Length > 0)
        {
            Array.Copy(payload, 0, data, tcpOffset + tcpHeaderLength, payload.Length);
        }

        return new RawPacket
        {
            SourceIp = srcIp,
            SourcePort = srcPort,
            DestinationIp = dstIp,
            DestinationPort = dstPort,
            Protocol = 6,
            Data = data
        };
    }

    private static readonly IPAddress ClientIp = IPAddress.Parse("192.168.1.100");
    private static readonly IPAddress ServerIp = IPAddress.Parse("10.0.0.1");
    private const ushort ClientPort = 12345;
    private const ushort ServerPort = 80;

    [Fact]
    public void ProcessPacket_SynPacket_CreatesNewSession()
    {
        var synPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1000, ackNum: 0, syn: true);

        _reassembler.ProcessPacket(synPacket);

        Assert.Equal(1, _reassembler.ActiveSessionCount);
        var sessions = _reassembler.GetSessions();
        var session = sessions.Values.First();
        Assert.Equal(TcpState.SynSent, session.State);
        Assert.Equal((uint)1001, session.ClientSeq);
    }

    [Fact]
    public void ProcessPacket_InOrderData_DeliveredCorrectly()
    {
        var synPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1000, ackNum: 0, syn: true);
        _reassembler.ProcessPacket(synPacket);

        var dataPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1001, ackNum: 0, ack: true,
            payload: System.Text.Encoding.UTF8.GetBytes("Hello"));

        _reassembler.ProcessPacket(dataPacket);

        var session = _reassembler.GetSessions().Values.First();
        Assert.Equal("Hello", System.Text.Encoding.UTF8.GetString(session.ClientData.ToArray()));
    }

    [Fact]
    public void ProcessPacket_OutOfOrderData_ReassembledWhenInOrderArrives()
    {
        var synPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1000, ackNum: 0, syn: true);
        _reassembler.ProcessPacket(synPacket);

        var outOfOrderPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1007, ackNum: 0, ack: true,
            payload: System.Text.Encoding.UTF8.GetBytes("World"));

        _reassembler.ProcessPacket(outOfOrderPacket);

        var inOrderPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1001, ackNum: 0, ack: true,
            payload: System.Text.Encoding.UTF8.GetBytes("Hello "));

        _reassembler.ProcessPacket(inOrderPacket);

        var session = _reassembler.GetSessions().Values.First();
        Assert.Equal("Hello World", System.Text.Encoding.UTF8.GetString(session.ClientData.ToArray()));
    }

    [Fact]
    public void ProcessPacket_FinPacket_ClosesSession()
    {
        var synPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1000, ackNum: 0, syn: true);
        _reassembler.ProcessPacket(synPacket);

        var dataPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1001, ackNum: 0, ack: true,
            payload: System.Text.Encoding.UTF8.GetBytes("Hello"));
        _reassembler.ProcessPacket(dataPacket);

        TcpReassemblySession? closedSession = null;
        _reassembler.SessionClosed += (s, e) => closedSession = e;

        var finPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1006, ackNum: 0, fin: true, ack: true);
        _reassembler.ProcessPacket(finPacket);

        Assert.NotNull(closedSession);
        var session = _reassembler.GetSessions().Values.First();
        Assert.True(session.State == TcpState.FinWait1 || session.State == TcpState.FinWait2);
    }

    [Fact]
    public void ProcessPacket_RstPacket_ResetsSession()
    {
        var synPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1000, ackNum: 0, syn: true);
        _reassembler.ProcessPacket(synPacket);

        var dataPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1001, ackNum: 0, ack: true,
            payload: System.Text.Encoding.UTF8.GetBytes("Hello"));
        _reassembler.ProcessPacket(dataPacket);

        TcpReassemblySession? closedSession = null;
        _reassembler.SessionClosed += (s, e) => closedSession = e;

        var rstPacket = BuildTcpPacket(ClientIp, ClientPort, ServerIp, ServerPort,
            seqNum: 1006, ackNum: 0, rst: true);
        _reassembler.ProcessPacket(rstPacket);

        Assert.NotNull(closedSession);
        var session = _reassembler.GetSessions().Values.First();
        Assert.Equal(TcpState.Closed, session.State);
    }
}
