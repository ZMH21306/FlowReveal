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

        private long _lastStatsPackets;
        private DateTime _lastStatsTime;
        public double InstantPacketsPerSecond { get; set; }

        public TimeSpan CaptureDuration => CaptureEndTime > CaptureStartTime ? CaptureEndTime - CaptureStartTime : TimeSpan.Zero;
        public double PacketsPerSecond => InstantPacketsPerSecond > 0 ? InstantPacketsPerSecond : (CaptureDuration.TotalSeconds > 0 ? TotalPacketsCaptured / CaptureDuration.TotalSeconds : 0);
        public double BytesPerSecond => CaptureDuration.TotalSeconds > 0 ? TotalBytesCaptured / CaptureDuration.TotalSeconds : 0;

        public void UpdateInstantRate()
        {
            var now = DateTime.UtcNow;
            var elapsed = (now - _lastStatsTime).TotalSeconds;
            if (elapsed >= 1.0 && _lastStatsTime != DateTime.MinValue)
            {
                var deltaPackets = TotalPacketsCaptured - _lastStatsPackets;
                InstantPacketsPerSecond = deltaPackets / elapsed;
                _lastStatsPackets = TotalPacketsCaptured;
                _lastStatsTime = now;
            }
            else if (_lastStatsTime == DateTime.MinValue)
            {
                _lastStatsPackets = TotalPacketsCaptured;
                _lastStatsTime = now;
            }
        }
    }
}
