using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Net;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Services.Parser
{
    public class TcpSessionKey : IEquatable<TcpSessionKey>
    {
        public IPAddress SourceIp { get; }
        public ushort SourcePort { get; }
        public IPAddress DestinationIp { get; }
        public ushort DestinationPort { get; }

        public TcpSessionKey(IPAddress srcIp, ushort srcPort, IPAddress dstIp, ushort dstPort)
        {
            SourceIp = srcIp;
            SourcePort = srcPort;
            DestinationIp = dstIp;
            DestinationPort = dstPort;
        }

        public TcpSessionKey Reversed => new(DestinationIp, DestinationPort, SourceIp, SourcePort);

        public bool Equals(TcpSessionKey? other)
        {
            if (other is null) return false;
            return SourceIp.Equals(other.SourceIp) &&
                   SourcePort == other.SourcePort &&
                   DestinationIp.Equals(other.DestinationIp) &&
                   DestinationPort == other.DestinationPort;
        }

        public override bool Equals(object? obj) => Equals(obj as TcpSessionKey);

        public override int GetHashCode() => HashCode.Combine(SourceIp, SourcePort, DestinationIp, DestinationPort);

        public override string ToString() => $"{SourceIp}:{SourcePort} -> {DestinationIp}:{DestinationPort}";
    }

    public class TcpReassemblySession
    {
        public TcpSessionKey Key { get; }
        public TcpSessionKey ReverseKey { get; }
        public TcpState State { get; set; } = TcpState.Closed;
        public DateTime StartTime { get; set; } = DateTime.UtcNow;
        public DateTime LastActivityTime { get; set; } = DateTime.UtcNow;
        public uint ClientSeq { get; set; }
        public uint ServerSeq { get; set; }
        public List<byte> ClientData { get; } = new();
        public List<byte> ServerData { get; } = new();
        public SortedList<uint, byte[]> ClientOutOfOrder { get; } = new();
        public SortedList<uint, byte[]> ServerOutOfOrder { get; } = new();
        public long ClientBytesReceived { get; set; }
        public long ServerBytesReceived { get; set; }

        public TcpReassemblySession(TcpSessionKey key)
        {
            Key = key;
            ReverseKey = key.Reversed;
        }
    }

    public class TcpReassembler
    {
        private readonly ILogger<TcpReassembler> _logger;
        private readonly ConcurrentDictionary<TcpSessionKey, TcpReassemblySession> _sessions = new();
        private readonly TimeSpan _sessionTimeout = TimeSpan.FromMinutes(5);
        private readonly int _maxSessionBufferSize = 10 * 1024 * 1024;

        public event EventHandler<TcpReassemblySession>? SessionEstablished;
        public event EventHandler<TcpReassemblySession>? SessionDataReceived;
        public event EventHandler<TcpReassemblySession>? SessionClosed;

        public int ActiveSessionCount => _sessions.Count;

        public TcpReassembler(ILogger<TcpReassembler> logger)
        {
            _logger = logger;
        }

        public void ProcessPacket(RawPacket packet)
        {
            if (packet.Protocol != 6)
                return;

            if (packet.Data.Length < 20)
                return;

            int ipHeaderLength = (packet.Data[0] & 0x0F) * 4;
            if (packet.Data.Length < ipHeaderLength + 20)
                return;

            int tcpOffset = ipHeaderLength;
            byte tcpFlags = packet.Data[tcpOffset + 13];
            uint seqNumber = ((uint)packet.Data[tcpOffset + 4] << 24) |
                             ((uint)packet.Data[tcpOffset + 5] << 16) |
                             ((uint)packet.Data[tcpOffset + 6] << 8) |
                             packet.Data[tcpOffset + 7];
            uint ackNumber = ((uint)packet.Data[tcpOffset + 8] << 24) |
                             ((uint)packet.Data[tcpOffset + 9] << 16) |
                             ((uint)packet.Data[tcpOffset + 10] << 8) |
                             packet.Data[tcpOffset + 11];
            int tcpHeaderLength = ((packet.Data[tcpOffset + 12] >> 4) & 0x0F) * 4;

            int payloadOffset = tcpOffset + tcpHeaderLength;
            int payloadLength = packet.Data.Length - payloadOffset;

            bool isSyn = (tcpFlags & 0x02) != 0;
            bool isAck = (tcpFlags & 0x10) != 0;
            bool isFin = (tcpFlags & 0x01) != 0;
            bool isRst = (tcpFlags & 0x04) != 0;

            var key = new TcpSessionKey(packet.SourceIp, packet.SourcePort, packet.DestinationIp, packet.DestinationPort);

            if (isSyn && !isAck)
            {
                HandleSyn(key, seqNumber);
                return;
            }

            var session = FindOrCreateSession(key);
            if (session == null)
                return;

            session.LastActivityTime = DateTime.UtcNow;

            bool isClient = key.Equals(session.Key);

            if (isRst)
            {
                HandleRst(session);
                return;
            }

            if (isFin)
            {
                HandleFin(session, isClient, seqNumber);
                return;
            }

            if (payloadLength > 0)
            {
                var payload = new byte[payloadLength];
                Array.Copy(packet.Data, payloadOffset, payload, 0, payloadLength);
                HandleData(session, isClient, seqNumber, payload);
            }
        }

        private void HandleSyn(TcpSessionKey key, uint seqNumber)
        {
            var session = new TcpReassemblySession(key)
            {
                State = TcpState.SynSent,
                ClientSeq = seqNumber + 1
            };

            _sessions[key] = session;

            _logger.LogDebug("TCP SYN: {Key} (ISN={ISN})", key, seqNumber);
        }

        private TcpReassemblySession? FindOrCreateSession(TcpSessionKey key)
        {
            if (_sessions.TryGetValue(key, out var session))
                return session;

            var reverseKey = key.Reversed;
            foreach (var kvp in _sessions)
            {
                if (kvp.Key.Equals(reverseKey))
                    return kvp.Value;
            }

            return null;
        }

        private void HandleData(TcpReassemblySession session, bool isClient, uint seqNumber, byte[] payload)
        {
            var expectedSeq = isClient ? session.ClientSeq : session.ServerSeq;
            var buffer = isClient ? session.ClientData : session.ServerData;
            var outOfOrder = isClient ? session.ClientOutOfOrder : session.ServerOutOfOrder;

            if (buffer.Count >= _maxSessionBufferSize)
            {
                _logger.LogWarning("Session {Key} buffer exceeded max size, dropping data", session.Key);
                return;
            }

            if (seqNumber == expectedSeq)
            {
                buffer.AddRange(payload);
                if (isClient)
                {
                    session.ClientSeq = seqNumber + (uint)payload.Length;
                    session.ClientBytesReceived += payload.Length;
                }
                else
                {
                    session.ServerSeq = seqNumber + (uint)payload.Length;
                    session.ServerBytesReceived += payload.Length;
                }

                ProcessOutOfOrderData(session, isClient);

                if (session.State == TcpState.Established)
                {
                    SessionDataReceived?.Invoke(this, session);
                }
            }
            else if (IsSequenceAfter(seqNumber, expectedSeq))
            {
                outOfOrder[seqNumber] = payload;
                _logger.LogDebug("Out-of-order segment: {Key} seq={Seq} expected={Expected} (isClient={IsClient})",
                    session.Key, seqNumber, expectedSeq, isClient);
            }

            if (session.State == TcpState.SynSent || session.State == TcpState.SynReceived)
            {
                session.State = TcpState.Established;
                _logger.LogInformation("TCP session established: {Key}", session.Key);
                SessionEstablished?.Invoke(this, session);
            }
        }

        private void ProcessOutOfOrderData(TcpReassemblySession session, bool isClient)
        {
            var expectedSeq = isClient ? session.ClientSeq : session.ServerSeq;
            var buffer = isClient ? session.ClientData : session.ServerData;
            var outOfOrder = isClient ? session.ClientOutOfOrder : session.ServerOutOfOrder;

            var keysToRemove = new List<uint>();
            foreach (var kvp in outOfOrder)
            {
                if (kvp.Key == expectedSeq)
                {
                    buffer.AddRange(kvp.Value);
                    expectedSeq += (uint)kvp.Value.Length;
                    if (isClient)
                    {
                        session.ClientSeq = expectedSeq;
                        session.ClientBytesReceived += kvp.Value.Length;
                    }
                    else
                    {
                        session.ServerSeq = expectedSeq;
                        session.ServerBytesReceived += kvp.Value.Length;
                    }
                    keysToRemove.Add(kvp.Key);
                }
                else if (!IsSequenceAfter(kvp.Key, expectedSeq))
                {
                    keysToRemove.Add(kvp.Key);
                }
                else
                {
                    break;
                }
            }

            foreach (var k in keysToRemove)
            {
                outOfOrder.Remove(k);
            }
        }

        private void HandleFin(TcpReassemblySession session, bool isClient, uint seqNumber)
        {
            _logger.LogDebug("TCP FIN: {Key} (isClient={IsClient})", session.Key, isClient);

            if (session.State == TcpState.Established)
                session.State = TcpState.FinWait1;
            else if (session.State == TcpState.FinWait1)
                session.State = TcpState.FinWait2;
            else if (session.State == TcpState.CloseWait)
                session.State = TcpState.LastAck;

            CloseSession(session);
        }

        private void HandleRst(TcpReassemblySession session)
        {
            _logger.LogInformation("TCP RST: {Key}", session.Key);
            session.State = TcpState.Closed;
            CloseSession(session);
        }

        private void CloseSession(TcpReassemblySession session)
        {
            SessionClosed?.Invoke(this, session);
            _logger.LogInformation("TCP session closed: {Key} (ClientBytes={ClientBytes}, ServerBytes={ServerBytes})",
                session.Key, session.ClientBytesReceived, session.ServerBytesReceived);
        }

        private bool IsSequenceAfter(uint seq1, uint seq2)
        {
            return seq1 != seq2 && (seq1 - seq2) < uint.MaxValue / 2;
        }

        public void CleanupExpiredSessions()
        {
            var now = DateTime.UtcNow;
            var expiredKeys = new List<TcpSessionKey>();

            foreach (var kvp in _sessions)
            {
                if (now - kvp.Value.LastActivityTime > _sessionTimeout)
                {
                    expiredKeys.Add(kvp.Key);
                }
            }

            foreach (var key in expiredKeys)
            {
                if (_sessions.TryRemove(key, out var session))
                {
                    _logger.LogInformation("TCP session expired: {Key}", key);
                    SessionClosed?.Invoke(this, session);
                }
            }

            if (expiredKeys.Count > 0)
            {
                _logger.LogInformation("Cleaned up {Count} expired TCP sessions", expiredKeys.Count);
            }
        }

        public void Clear()
        {
            _sessions.Clear();
            _logger.LogInformation("All TCP sessions cleared");
        }

        public IReadOnlyDictionary<TcpSessionKey, TcpReassemblySession> GetSessions() => _sessions;
    }
}
