using System;

namespace FlowReveal.Core.Models
{
    public class CaptureStatistics
    {
        public long TotalPacketsCaptured { get; set; }
        public long TotalPacketsDropped { get; set; }
        public long TotalBytesCaptured { get; set; }
        public long TotalHttpConversations { get; set; }
        public long ActiveTcpSessions { get; set; }
        public DateTime CaptureStartTime { get; set; } = DateTime.MinValue;
        public DateTime CaptureEndTime { get; set; } = DateTime.MinValue;
        public TimeSpan CaptureDuration => CaptureEndTime > CaptureStartTime ? CaptureEndTime - CaptureStartTime : TimeSpan.Zero;
        public double PacketsPerSecond => CaptureDuration.TotalSeconds > 0 ? TotalPacketsCaptured / CaptureDuration.TotalSeconds : 0;
        public double BytesPerSecond => CaptureDuration.TotalSeconds > 0 ? TotalBytesCaptured / CaptureDuration.TotalSeconds : 0;
    }
}
