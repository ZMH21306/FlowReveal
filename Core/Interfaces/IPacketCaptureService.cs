using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using FlowReveal.Core.Models;

namespace FlowReveal.Core.Interfaces
{
    public enum CaptureEngineType
    {
        RawSocket,
        Wfp
    }

    public interface IPacketCaptureService : IDisposable
    {
        event EventHandler<RawPacket>? PacketCaptured;
        event EventHandler<CaptureStatistics>? StatisticsUpdated;
        event EventHandler<string>? StatusChanged;
        bool IsCapturing { get; }
        CaptureEngineType EngineType { get; }
        CaptureStatistics Statistics { get; }
        IReadOnlyList<NetworkAdapter> AvailableAdapters { get; }
        Task StartCaptureAsync(NetworkAdapter adapter, CancellationToken cancellationToken = default);
        Task StopCaptureAsync();
    }
}
