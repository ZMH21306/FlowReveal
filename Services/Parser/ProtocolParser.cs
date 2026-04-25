#pragma warning disable CS0067
using System;
using System.Collections.Generic;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace FlowReveal.Services.Parser
{
    public class ProtocolParser : IProtocolParser
    {
        private readonly ILogger<ProtocolParser> _logger;
        private readonly TcpReassembler _tcpReassembler;
        private readonly List<HttpConversation> _conversations = new();
        private readonly Dictionary<TcpSessionKey, PendingRequest> _pendingRequests = new();

        public event EventHandler<HttpConversation>? ConversationCreated;
        public event EventHandler<HttpConversation>? ConversationUpdated;

        public ProtocolParser(ILogger<ProtocolParser> logger)
        {
            _logger = logger;
            _tcpReassembler = new TcpReassembler(NullLogger<TcpReassembler>.Instance);

            _tcpReassembler.SessionDataReceived += OnSessionDataReceived;
            _tcpReassembler.SessionClosed += OnSessionClosed;
        }

        public void ProcessPacket(RawPacket packet)
        {
            if (packet.Protocol == 6)
            {
                _tcpReassembler.ProcessPacket(packet);
            }
        }

        private void OnSessionDataReceived(object? sender, TcpReassemblySession session)
        {
            TryParseHttpFromSession(session);
        }

        private void OnSessionClosed(object? sender, TcpReassemblySession session)
        {
            TryParseHttpFromSession(session);

            if (_pendingRequests.TryGetValue(session.Key, out var pending) ||
                _pendingRequests.TryGetValue(session.ReverseKey, out pending))
            {
                if (pending.Conversation != null && !pending.Conversation.HasResponse)
                {
                    pending.Conversation.EndTime = DateTime.UtcNow;
                    ConversationUpdated?.Invoke(this, pending.Conversation);
                    _logger.LogDebug("HTTP conversation closed without response: {Method} {Url}",
                        pending.Conversation.Request.Method, pending.Conversation.Request.Url);
                }
            }
        }

        private void TryParseHttpFromSession(TcpReassemblySession session)
        {
            var key = session.Key;

            while (session.ClientData.Count > 0)
            {
                var clientData = session.ClientData.ToArray();

                if (!HttpParser.LooksLikeHttpRequest(clientData, 0, clientData.Length))
                    break;

                if (!HttpParser.TryParseRequest(clientData, 0, clientData.Length, out var request, out var consumedBytes) || request == null)
                    break;

                session.ClientData.RemoveRange(0, consumedBytes);

                var conversation = new HttpConversation
                {
                    StartTime = DateTime.UtcNow,
                    Request = new HttpRequest
                    {
                        Method = request.Method,
                        Url = request.Url,
                        Path = request.Path,
                        QueryString = request.QueryString,
                        HttpVersion = request.HttpVersion,
                        Headers = request.Headers,
                        Body = request.Body
                    },
                    IsHttps = key.DestinationPort == 443 || key.SourcePort == 443
                };

                _conversations.Add(conversation);
                _pendingRequests[key] = new PendingRequest { Conversation = conversation };

                _logger.LogInformation("HTTP Request: {Method} {Url} (Host: {Host})",
                    request.Method, request.Url,
                    request.Headers.TryGetValue("Host", out var host) ? host : "unknown");

                ConversationCreated?.Invoke(this, conversation);
            }

            while (session.ServerData.Count > 0)
            {
                var serverData = session.ServerData.ToArray();

                if (!HttpParser.LooksLikeHttpResponse(serverData, 0, serverData.Length))
                    break;

                if (!HttpParser.TryParseResponse(serverData, 0, serverData.Length, out var response, out var consumedBytes) || response == null)
                    break;

                session.ServerData.RemoveRange(0, consumedBytes);

                PendingRequest? pending = null;
                if (_pendingRequests.TryGetValue(key, out pending)) { }
                else if (_pendingRequests.TryGetValue(key.Reversed, out pending)) { }

                if (pending?.Conversation != null)
                {
                    var contentEncoding = response.Headers.GetValueOrDefault("Content-Encoding", string.Empty);
                    var decodedBody = HttpParser.DecodeContent(response.Body, contentEncoding);

                    pending.Conversation.Response = new HttpResponse
                    {
                        HttpVersion = response.HttpVersion,
                        StatusCode = response.StatusCode,
                        StatusDescription = response.StatusDescription,
                        Headers = response.Headers,
                        Body = decodedBody
                    };
                    pending.Conversation.EndTime = DateTime.UtcNow;

                    _logger.LogInformation("HTTP Response: {StatusCode} {StatusDescription} ({Size} bytes, Duration: {Duration}ms)",
                        response.StatusCode, response.StatusDescription,
                        decodedBody.Length,
                        (int)pending.Conversation.Duration.TotalMilliseconds);

                    ConversationUpdated?.Invoke(this, pending.Conversation);
                }
            }
        }

        public IReadOnlyList<HttpConversation> GetConversations() => _conversations.AsReadOnly();

        public void Clear()
        {
            _conversations.Clear();
            _pendingRequests.Clear();
            _tcpReassembler.Clear();
            _logger.LogInformation("All conversations and sessions cleared");
        }

        private class PendingRequest
        {
            public HttpConversation? Conversation { get; set; }
        }
    }
}
