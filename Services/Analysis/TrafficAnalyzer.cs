using System;
using System.Collections.Generic;
using System.Linq;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Services.Analysis
{
    public class ProtocolDistribution
    {
        public int HttpCount { get; set; }
        public int HttpsCount { get; set; }
        public double HttpPercentage => TotalCount > 0 ? (double)HttpCount / TotalCount * 100 : 0;
        public double HttpsPercentage => TotalCount > 0 ? (double)HttpsCount / TotalCount * 100 : 0;
        public int TotalCount => HttpCount + HttpsCount;
    }

    public class MethodDistribution : Dictionary<string, int> { }

    public class StatusCodeDistribution : Dictionary<int, int> { }

    public class BandwidthSample
    {
        public DateTime Timestamp { get; set; }
        public long BytesPerSecond { get; set; }
        public int RequestsPerSecond { get; set; }
    }

    public class SlowRequestInfo
    {
        public HttpConversation Conversation { get; set; } = null!;
        public double DurationMs { get; set; }
    }

    public class TrafficAnalysisResult
    {
        public ProtocolDistribution ProtocolDistribution { get; set; } = new();
        public MethodDistribution MethodDistribution { get; set; } = new();
        public StatusCodeDistribution StatusCodeDistribution { get; set; } = new();
        public List<BandwidthSample> BandwidthHistory { get; set; } = new();
        public List<SlowRequestInfo> SlowRequests { get; set; } = new();
        public int TotalConversations { get; set; }
        public int ErrorCount { get; set; }
        public double ErrorRate => TotalConversations > 0 ? (double)ErrorCount / TotalConversations * 100 : 0;
        public double AverageResponseTimeMs { get; set; }
        public double MedianResponseTimeMs { get; set; }
        public double P95ResponseTimeMs { get; set; }
        public long TotalBytesTransferred { get; set; }
        public double AverageRequestSize { get; set; }
        public double AverageResponseSize { get; set; }
    }

    public class TrafficAnalyzer
    {
        private readonly ILogger<TrafficAnalyzer> _logger;
        private readonly List<BandwidthSample> _bandwidthHistory = new();
        private DateTime _lastSampleTime = DateTime.UtcNow;
        private long _bytesSinceLastSample;
        private int _requestsSinceLastSample;

        public double SlowRequestThresholdMs { get; set; } = 3000;

        public TrafficAnalyzer(ILogger<TrafficAnalyzer> logger)
        {
            _logger = logger;
        }

        public TrafficAnalysisResult Analyze(IEnumerable<HttpConversation> conversations)
        {
            var convList = conversations.ToList();
            var result = new TrafficAnalysisResult
            {
                TotalConversations = convList.Count
            };

            if (convList.Count == 0)
            {
                _logger.LogDebug("No conversations to analyze");
                return result;
            }

            AnalyzeProtocolDistribution(convList, result);
            AnalyzeMethodDistribution(convList, result);
            AnalyzeStatusCodeDistribution(convList, result);
            AnalyzeResponseTimes(convList, result);
            AnalyzeDataSizes(convList, result);
            AnalyzeErrors(convList, result);
            FindSlowRequests(convList, result);

            result.BandwidthHistory = new List<BandwidthSample>(_bandwidthHistory);

            _logger.LogInformation("Traffic analysis: {Total} conversations, {Errors} errors ({ErrorRate:F1}%), avg response {AvgMs:F1}ms",
                result.TotalConversations, result.ErrorCount, result.ErrorRate, result.AverageResponseTimeMs);

            return result;
        }

        public void RecordBytesTransferred(long bytes)
        {
            _bytesSinceLastSample += bytes;
            _requestsSinceLastSample++;

            var now = DateTime.UtcNow;
            if ((now - _lastSampleTime).TotalSeconds >= 1)
            {
                _bandwidthHistory.Add(new BandwidthSample
                {
                    Timestamp = _lastSampleTime,
                    BytesPerSecond = _bytesSinceLastSample,
                    RequestsPerSecond = _requestsSinceLastSample
                });

                if (_bandwidthHistory.Count > 3600)
                {
                    _bandwidthHistory.RemoveAt(0);
                }

                _bytesSinceLastSample = 0;
                _requestsSinceLastSample = 0;
                _lastSampleTime = now;
            }
        }

        private void AnalyzeProtocolDistribution(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            var protocol = new ProtocolDistribution();
            foreach (var conv in conversations)
            {
                if (conv.IsHttps)
                    protocol.HttpsCount++;
                else
                    protocol.HttpCount++;
            }
            result.ProtocolDistribution = protocol;
        }

        private void AnalyzeMethodDistribution(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            var methods = new MethodDistribution();
            foreach (var conv in conversations)
            {
                var method = conv.Request.Method;
                if (!methods.ContainsKey(method))
                    methods[method] = 0;
                methods[method]++;
            }
            result.MethodDistribution = methods;
        }

        private void AnalyzeStatusCodeDistribution(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            var statuses = new StatusCodeDistribution();
            foreach (var conv in conversations)
            {
                if (conv.HasResponse)
                {
                    var code = conv.Response.StatusCode;
                    if (!statuses.ContainsKey(code))
                        statuses[code] = 0;
                    statuses[code]++;
                }
            }
            result.StatusCodeDistribution = statuses;
        }

        private void AnalyzeResponseTimes(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            var completedConversations = conversations.Where(c => c.HasResponse && c.Duration > TimeSpan.Zero).ToList();

            if (completedConversations.Count == 0) return;

            var responseTimes = completedConversations.Select(c => c.Duration.TotalMilliseconds).OrderBy(t => t).ToList();

            result.AverageResponseTimeMs = responseTimes.Average();
            result.MedianResponseTimeMs = responseTimes[responseTimes.Count / 2];
            result.P95ResponseTimeMs = responseTimes[(int)(responseTimes.Count * 0.95)];
        }

        private void AnalyzeDataSizes(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            result.TotalBytesTransferred = conversations.Sum(c => c.TotalSize);

            var withResponse = conversations.Where(c => c.HasResponse).ToList();
            if (withResponse.Count > 0)
            {
                result.AverageRequestSize = withResponse.Average(c => c.Request.Body.Length);
                result.AverageResponseSize = withResponse.Average(c => c.Response.Body.Length);
            }
        }

        private void AnalyzeErrors(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            result.ErrorCount = conversations.Count(c => c.IsError);
        }

        private void FindSlowRequests(List<HttpConversation> conversations, TrafficAnalysisResult result)
        {
            result.SlowRequests = conversations
                .Where(c => c.Duration.TotalMilliseconds > SlowRequestThresholdMs)
                .Select(c => new SlowRequestInfo
                {
                    Conversation = c,
                    DurationMs = c.Duration.TotalMilliseconds
                })
                .OrderByDescending(s => s.DurationMs)
                .Take(100)
                .ToList();
        }
    }
}
