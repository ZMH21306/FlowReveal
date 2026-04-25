using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Authentication;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using FlowReveal.Core.Models;
using FlowReveal.Services.Parser;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Platforms.Windows.Security
{
    public class HttpsProxyServer
    {
        private readonly ILogger<HttpsProxyServer> _logger;
        private readonly CertificateManager _certificateManager;
        private TcpListener? _listener;
        private CancellationTokenSource? _cts;
        private Task? _listenTask;
        private bool _isRunning;
        private int _port;

        private readonly ConcurrentDictionary<string, X509Certificate2> _siteCertificates = new();

        public event EventHandler<HttpConversation>? ConversationCaptured;
        public event EventHandler<string>? ProxyStatusChanged;

        public bool IsRunning => _isRunning;
        public int Port => _port;
        public int ActiveConnections => _activeConnections;

        private int _activeConnections;

        public HttpsProxyServer(
            ILogger<HttpsProxyServer> logger,
            CertificateManager certificateManager)
        {
            _logger = logger;
            _certificateManager = certificateManager;
        }

        public async Task StartAsync(int port = 8888, CancellationToken cancellationToken = default)
        {
            if (_isRunning)
            {
                _logger.LogWarning("Proxy server is already running");
                return;
            }

            _port = port;
            _cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);

            try
            {
                _listener = new TcpListener(IPAddress.Loopback, _port);
                _listener.Start();

                _isRunning = true;
                ProxyStatusChanged?.Invoke(this, $"Proxy running on 127.0.0.1:{_port}");

                _logger.LogInformation("HTTPS proxy server started on port {Port}", _port);

                _listenTask = Task.Run(() => AcceptLoopAsync(_cts.Token), _cts.Token);

                await Task.CompletedTask;
            }
            catch (Exception ex)
            {
                _isRunning = false;
                _logger.LogError(ex, "Failed to start proxy server on port {Port}", _port);
                throw;
            }
        }

        public async Task StopAsync()
        {
            if (!_isRunning) return;

            _logger.LogInformation("Stopping HTTPS proxy server...");
            _isRunning = false;
            _cts?.Cancel();

            _listener?.Stop();

            if (_listenTask != null)
            {
                try { await _listenTask; }
                catch (OperationCanceledException) { }
                catch (Exception ex) { _logger.LogWarning(ex, "Error waiting for listen task"); }
            }

            foreach (var cert in _siteCertificates.Values)
            {
                cert.Dispose();
            }
            _siteCertificates.Clear();

            _cts?.Dispose();
            _cts = null;

            ProxyStatusChanged?.Invoke(this, "Proxy stopped");
            _logger.LogInformation("HTTPS proxy server stopped");
        }

        private async Task AcceptLoopAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested && _isRunning)
            {
                try
                {
                    var client = await _listener!.AcceptTcpClientAsync(cancellationToken);
                    _ = HandleClientAsync(client, cancellationToken);
                }
                catch (OperationCanceledException) { break; }
                catch (Exception ex)
                {
                    if (_isRunning)
                        _logger.LogWarning(ex, "Error accepting connection");
                }
            }
        }

        private async Task HandleClientAsync(TcpClient client, CancellationToken cancellationToken)
        {
            Interlocked.Increment(ref _activeConnections);
            var clientEndpoint = client.Client.RemoteEndPoint?.ToString() ?? "unknown";

            try
            {
                using var stream = client.GetStream();
                var buffer = new byte[8192];
                var bytesRead = await stream.ReadAsync(buffer.AsMemory(0, buffer.Length), cancellationToken);

                if (bytesRead == 0) return;

                var requestData = buffer[..bytesRead];
                var requestText = Encoding.ASCII.GetString(requestData);

                if (requestText.StartsWith("CONNECT", StringComparison.OrdinalIgnoreCase))
                {
                    await HandleHttpsTunnelAsync(stream, requestText, cancellationToken);
                }
                else
                {
                    await HandleHttpProxyAsync(stream, requestText, requestData, bytesRead, cancellationToken);
                }
            }
            catch (Exception ex)
            {
                _logger.LogDebug(ex, "Error handling client connection from {Endpoint}", clientEndpoint);
            }
            finally
            {
                Interlocked.Decrement(ref _activeConnections);
                client.Dispose();
            }
        }

        private async Task HandleHttpsTunnelAsync(NetworkStream clientStream, string connectRequest, CancellationToken cancellationToken)
        {
            var parts = connectRequest.Split(' ');
            if (parts.Length < 2) return;

            var hostPort = parts[1].Split(':');
            var hostname = hostPort[0];
            var port = hostPort.Length > 1 ? int.Parse(hostPort[1]) : 443;

            _logger.LogDebug("HTTPS tunnel request: {Host}:{Port}", hostname, port);

            var connectResponse = Encoding.ASCII.GetBytes("HTTP/1.1 200 Connection Established\r\n\r\n");
            await clientStream.WriteAsync(connectResponse, cancellationToken);

            X509Certificate2 siteCert;
            try
            {
                siteCert = _siteCertificates.GetOrAdd(hostname, h => _certificateManager.IssueSiteCertificate(h));
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Failed to issue certificate for {Hostname}", hostname);
                return;
            }

            try
            {
                using var sslStream = new SslStream(clientStream, false);
                await sslStream.AuthenticateAsServerAsync(siteCert, false, SslProtocols.Tls12 | SslProtocols.Tls13, false);

                _logger.LogDebug("SSL tunnel established for {Hostname}", hostname);

                using var remoteClient = new TcpClient();
                await remoteClient.ConnectAsync(hostname, port, cancellationToken);

                using var remoteStream = remoteClient.GetStream();
                using var remoteSsl = new SslStream(remoteStream, false);
                await remoteSsl.AuthenticateAsClientAsync(hostname, null, SslProtocols.Tls12 | SslProtocols.Tls13, false);

                var clientBuffer = new List<byte>();
                var serverBuffer = new List<byte>();

                var relayTask1 = RelayDataAsync(sslStream, remoteSsl, clientBuffer, "Client->Server", cancellationToken);
                var relayTask2 = RelayDataAsync(remoteSsl, sslStream, serverBuffer, "Server->Client", cancellationToken);

                await Task.WhenAny(relayTask1, relayTask2);

                var conversation = TryParseHttpsConversation(hostname, clientBuffer, serverBuffer);
                if (conversation != null)
                {
                    ConversationCaptured?.Invoke(this, conversation);
                }
            }
            catch (AuthenticationException ex)
            {
                _logger.LogDebug(ex, "SSL authentication failed for {Hostname}", hostname);
            }
            catch (Exception ex)
            {
                _logger.LogDebug(ex, "HTTPS tunnel error for {Hostname}", hostname);
            }
        }

        private HttpConversation? TryParseHttpsConversation(string hostname, List<byte> clientData, List<byte> serverData)
        {
            try
            {
                var conversation = new HttpConversation
                {
                    StartTime = DateTime.UtcNow,
                    EndTime = DateTime.UtcNow,
                    IsHttps = true
                };

                if (clientData.Count > 0)
                {
                    var clientBytes = clientData.ToArray();
                    if (HttpParser.LooksLikeHttpRequest(clientBytes, 0, clientBytes.Length))
                    {
                        if (HttpParser.TryParseRequest(clientBytes, 0, clientBytes.Length, out var request, out _) && request != null)
                        {
                            conversation.Request = new HttpRequest
                            {
                                Method = request.Method,
                                Url = request.Url,
                                Path = request.Path,
                                QueryString = request.QueryString,
                                HttpVersion = request.HttpVersion,
                                Headers = request.Headers,
                                Body = request.Body
                            };
                        }
                    }
                }

                if (serverData.Count > 0)
                {
                    var serverBytes = serverData.ToArray();
                    if (HttpParser.LooksLikeHttpResponse(serverBytes, 0, serverBytes.Length))
                    {
                        if (HttpParser.TryParseResponse(serverBytes, 0, serverBytes.Length, out var response, out _) && response != null)
                        {
                            var contentEncoding = response.Headers.GetValueOrDefault("Content-Encoding", string.Empty);
                            var decodedBody = HttpParser.DecodeContent(response.Body, contentEncoding);

                            conversation.Response = new HttpResponse
                            {
                                HttpVersion = response.HttpVersion,
                                StatusCode = response.StatusCode,
                                StatusDescription = response.StatusDescription,
                                Headers = response.Headers,
                                Body = decodedBody
                            };
                        }
                    }
                }

                if (conversation.Request.Method.Length > 0 || conversation.Response.StatusCode != 0)
                {
                    _logger.LogInformation("HTTPS conversation captured: {Method} {Url} -> {Status}",
                        conversation.Request.Method, conversation.Request.Url, conversation.Response.StatusCode);
                    return conversation;
                }

                return null;
            }
            catch (Exception ex)
            {
                _logger.LogDebug(ex, "Failed to parse HTTPS conversation for {Hostname}", hostname);
                return null;
            }
        }

        private async Task RelayDataAsync(SslStream source, SslStream destination, List<byte> buffer, string direction, CancellationToken cancellationToken)
        {
            var buf = new byte[8192];
            try
            {
                while (!cancellationToken.IsCancellationRequested)
                {
                    var bytesRead = await source.ReadAsync(buf.AsMemory(0, buf.Length), cancellationToken);
                    if (bytesRead == 0) break;

                    buffer.AddRange(buf[..bytesRead]);
                    await destination.WriteAsync(buf.AsMemory(0, bytesRead), cancellationToken);
                    await destination.FlushAsync(cancellationToken);
                }
            }
            catch (IOException) { }
            catch (OperationCanceledException) { }
            catch (Exception ex)
            {
                _logger.LogDebug(ex, "Relay error ({Direction})", direction);
            }
        }

        private async Task HandleHttpProxyAsync(NetworkStream clientStream, string requestText, byte[] requestData, int requestLength, CancellationToken cancellationToken)
        {
            var lines = requestText.Split("\r\n");
            if (lines.Length == 0) return;

            var requestLine = lines[0];
            var parts = requestLine.Split(' ');
            if (parts.Length < 2) return;

            var method = parts[0];
            var url = parts[1];

            _logger.LogDebug("HTTP proxy request: {Method} {Url}", method, url);

            try
            {
                if (!Uri.TryCreate(url, UriKind.Absolute, out var uri)) return;

                using var remoteClient = new TcpClient();
                await remoteClient.ConnectAsync(uri.Host, uri.Port, cancellationToken);

                using var remoteStream = remoteClient.GetStream();
                await remoteStream.WriteAsync(requestData.AsMemory(0, requestLength), cancellationToken);
                await remoteStream.FlushAsync(cancellationToken);

                var buffer = new byte[65535];
                while (true)
                {
                    var bytesRead = await remoteStream.ReadAsync(buffer.AsMemory(0, buffer.Length), cancellationToken);
                    if (bytesRead == 0) break;

                    await clientStream.WriteAsync(buffer.AsMemory(0, bytesRead), cancellationToken);
                    await clientStream.FlushAsync(cancellationToken);
                }
            }
            catch (Exception ex)
            {
                _logger.LogDebug(ex, "HTTP proxy error for {Url}", url);
            }
        }

        public bool SetSystemProxy(bool enable)
        {
            try
            {
                var proxyServer = enable ? $"127.0.0.1:{_port}" : "";
                var proxyEnable = enable ? 1 : 0;

                using var key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(
                    @"Software\Microsoft\Windows\CurrentVersion\Internet Settings", true);

                if (key == null)
                {
                    _logger.LogError("Failed to open Internet Settings registry key");
                    return false;
                }

                key.SetValue("ProxyServer", proxyServer);
                key.SetValue("ProxyEnable", proxyEnable);

                NativeMethods.InternetSetOption(IntPtr.Zero, NativeMethods.INTERNET_OPTION_SETTINGS_CHANGED, IntPtr.Zero, 0);
                NativeMethods.InternetSetOption(IntPtr.Zero, NativeMethods.INTERNET_OPTION_REFRESH, IntPtr.Zero, 0);

                _logger.LogInformation("System proxy {Status}: {Server}", enable ? "enabled" : "disabled", proxyServer);
                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to set system proxy");
                return false;
            }
        }

        private static class NativeMethods
        {
            public const int INTERNET_OPTION_SETTINGS_CHANGED = 39;
            public const int INTERNET_OPTION_REFRESH = 37;

            [System.Runtime.InteropServices.DllImport("wininet.dll")]
            public static extern bool InternetSetOption(IntPtr hInternet, int dwOption, IntPtr lpBuffer, int lpdwBufferLength);
        }
    }
}
