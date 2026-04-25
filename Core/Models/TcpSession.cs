using System;
using System.Collections.Generic;
using System.Net;

namespace FlowReveal.Core.Models
{
    public enum TcpState
    {
        Closed,
        SynSent,
        SynReceived,
        Established,
        FinWait1,
        FinWait2,
        CloseWait,
        Closing,
        LastAck,
        TimeWait
    }

    public class TcpSession
    {
        public Guid Id { get; } = Guid.NewGuid();
        public IPEndPoint Source { get; set; } = new IPEndPoint(IPAddress.None, 0);
        public IPEndPoint Destination { get; set; } = new IPEndPoint(IPAddress.None, 0);
        public TcpState State { get; set; } = TcpState.Closed;
        public DateTime StartTime { get; set; } = DateTime.UtcNow;
        public DateTime LastActivityTime { get; set; } = DateTime.UtcNow;
        public List<byte> ClientBuffer { get; } = new();
        public List<byte> ServerBuffer { get; } = new();
        public uint ClientSequenceNumber { get; set; }
        public uint ServerSequenceNumber { get; set; }
        public long ClientBytesReceived { get; set; }
        public long ServerBytesReceived { get; set; }
    }
}
