using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Services.Session
{
    public class SessionStore : ISessionStore
    {
        private readonly ILogger<SessionStore> _logger;
        private static readonly JsonSerializerOptions _jsonOptions = new()
        {
            WriteIndented = true,
            Converters = { new JsonStringEnumConverter() },
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };

        public SessionStore(ILogger<SessionStore> logger)
        {
            _logger = logger;
        }

        public async Task SaveSessionAsync(string filePath, IReadOnlyList<HttpConversation> conversations, CaptureStatistics statistics)
        {
            _logger.LogInformation("Saving session to {FilePath} with {Count} conversations", filePath, conversations.Count);

            try
            {
                var sessionData = new
                {
                    Version = "1.0",
                    SavedAt = DateTime.UtcNow,
                    Statistics = statistics,
                    Conversations = conversations
                };

                var json = JsonSerializer.Serialize(sessionData, _jsonOptions);
                await File.WriteAllTextAsync(filePath, json);

                _logger.LogInformation("Session saved successfully: {FilePath} ({Size} bytes)", filePath, new FileInfo(filePath).Length);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to save session to {FilePath}", filePath);
                throw;
            }
        }

        public async Task<(IReadOnlyList<HttpConversation> Conversations, CaptureStatistics Statistics)> LoadSessionAsync(string filePath)
        {
            _logger.LogInformation("Loading session from {FilePath}", filePath);

            try
            {
                var json = await File.ReadAllTextAsync(filePath);
                var sessionData = JsonSerializer.Deserialize<SessionData>(json, _jsonOptions);

                if (sessionData == null)
                {
                    throw new InvalidDataException("Failed to deserialize session data");
                }

                _logger.LogInformation("Session loaded: {Count} conversations, saved at {SavedAt}",
                    sessionData.Conversations?.Count ?? 0, sessionData.SavedAt);

                return ((IReadOnlyList<HttpConversation>)(sessionData.Conversations ?? new List<HttpConversation>()),
                        sessionData.Statistics ?? new CaptureStatistics());
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to load session from {FilePath}", filePath);
                throw;
            }
        }

        public async Task ExportJsonAsync(string filePath, IReadOnlyList<HttpConversation> conversations)
        {
            _logger.LogInformation("Exporting {Count} conversations to JSON: {FilePath}", conversations.Count, filePath);

            try
            {
                var exportData = conversations.Select(c => new
                {
                    c.Id,
                    c.StartTime,
                    c.EndTime,
                    c.Duration,
                    c.IsHttps,
                    c.Host,
                    c.TotalSize,
                    c.IsError,
                    c.IsSlow,
                    Request = new
                    {
                        c.Request.Method,
                        c.Request.Url,
                        c.Request.HttpVersion,
                        Headers = c.Request.Headers,
                        BodySize = c.Request.Body.Length
                    },
                    Response = c.HasResponse ? new
                    {
                        c.Response.StatusCode,
                        c.Response.StatusDescription,
                        c.Response.HttpVersion,
                        Headers = c.Response.Headers,
                        BodySize = c.Response.Body.Length
                    } : null
                });

                var json = JsonSerializer.Serialize(exportData, _jsonOptions);
                await File.WriteAllTextAsync(filePath, json);

                _logger.LogInformation("JSON export completed: {FilePath}", filePath);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to export JSON to {FilePath}", filePath);
                throw;
            }
        }

        public async Task ExportCsvAsync(string filePath, IReadOnlyList<HttpConversation> conversations)
        {
            _logger.LogInformation("Exporting {Count} conversations to CSV: {FilePath}", conversations.Count, filePath);

            try
            {
                var sb = new StringBuilder();
                sb.AppendLine("ID,Method,Host,URL,StatusCode,Size,Duration(ms),HTTPS,IsError,StartTime");

                foreach (var c in conversations)
                {
                    var method = EscapeCsvField(c.Request.Method);
                    var host = EscapeCsvField(c.Host);
                    var url = EscapeCsvField(c.Request.Url);
                    var status = c.HasResponse ? c.Response.StatusCode.ToString() : "";
                    var size = c.TotalSize.ToString();
                    var duration = c.Duration.TotalMilliseconds.ToString("F1", CultureInfo.InvariantCulture);
                    var https = c.IsHttps.ToString();
                    var error = c.IsError.ToString();
                    var startTime = c.StartTime.ToString("o");

                    sb.AppendLine($"{c.Id},{method},{host},{url},{status},{size},{duration},{https},{error},{startTime}");
                }

                await File.WriteAllTextAsync(filePath, sb.ToString());

                _logger.LogInformation("CSV export completed: {FilePath}", filePath);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to export CSV to {FilePath}", filePath);
                throw;
            }
        }

        public async Task ExportPcapAsync(string filePath, IReadOnlyList<RawPacket> packets)
        {
            _logger.LogInformation("Exporting {Count} packets to PCAP: {FilePath}", packets.Count, filePath);

            try
            {
                using var fs = new FileStream(filePath, FileMode.Create, FileAccess.Write);
                using var writer = new BinaryWriter(fs);

                writer.Write(0xa1b2c3d4);
                writer.Write((ushort)2);
                writer.Write((ushort)4);
                writer.Write(0);
                writer.Write(0);
                writer.Write((uint)65535);
                writer.Write((uint)1);

                foreach (var packet in packets)
                {
                    var ts = packet.Timestamp;
                    var tsSec = (uint)(ts - new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc)).TotalSeconds;
                    var tsUsec = (uint)ts.Millisecond * 1000;

                    writer.Write(tsSec);
                    writer.Write(tsUsec);
                    writer.Write((uint)packet.Length);
                    writer.Write((uint)packet.Length);
                    writer.Write(packet.Data);
                }

                _logger.LogInformation("PCAP export completed: {FilePath}", filePath);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to export PCAP to {FilePath}", filePath);
                throw;
            }
        }

        private static string EscapeCsvField(string field)
        {
            if (string.IsNullOrEmpty(field)) return "";
            if (field.Contains(',') || field.Contains('"') || field.Contains('\n'))
            {
                return $"\"{field.Replace("\"", "\"\"")}\"";
            }
            return field;
        }

        private class SessionData
        {
            [JsonPropertyName("version")]
            public string? Version { get; set; }

            [JsonPropertyName("savedAt")]
            public DateTime SavedAt { get; set; }

            [JsonPropertyName("statistics")]
            public CaptureStatistics? Statistics { get; set; }

            [JsonPropertyName("conversations")]
            public List<HttpConversation>? Conversations { get; set; }
        }
    }
}
