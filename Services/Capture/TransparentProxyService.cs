using FlowReveal.Models;
using FlowReveal.Services.Certificate;
using FlowReveal.Services.Http;
using FlowReveal.Services.Logging;
using System;
using System.IO;
using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Authentication;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace FlowReveal.Services.Capture;

public class TransparentProxyService : IDisposable
{
    private TcpListener? _httpListener;
    private TcpListener? _httpsListener;
    private readonly CertificateCache _certificateCache;
    private readonly HttpParser _httpParser;
    private readonly TlsHandshakeParser _tlsParser;
    private readonly int _httpPort;
    private readonly int _httpsPort;
    private bool _isRunning;
    private CancellationTokenSource? _cts;
    private Action<HttpTrafficRecord>? _onRecordCaptured;

    public TransparentProxyService(CertificateCache certificateCache, int httpPort = 9080, int httpsPort = 9443)
    {
        _certificateCache = certificateCache;
        _httpParser = new HttpParser();
        _tlsParser = new TlsHandshakeParser();
        _httpPort = httpPort;
        _httpsPort = httpsPort;
        Logger.LogInfo($"TransparentProxyService initialized with HTTP port {httpPort}, HTTPS port {httpsPort}");
    }

    public event Action<HttpTrafficRecord>? RecordCaptured
    {
        add => _onRecordCaptured += value;
        remove => _onRecordCaptured -= value;
    }

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        if (_isRunning)
        {
            Logger.LogWarning("Proxy service is already running");
            return;
        }

        _cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        _isRunning = true;

        try
        {
            _httpListener = new TcpListener(IPAddress.Loopback, _httpPort);
            _httpListener.Start();
            Logger.LogInfo($"HTTP listener started on 127.0.0.1:{_httpPort}");
            
            _httpsListener = new TcpListener(IPAddress.Loopback, _httpsPort);
            _httpsListener.Start();
            Logger.LogInfo($"HTTPS listener started on 127.0.0.1:{_httpsPort}");

            _ = AcceptHttpConnectionsAsync(_cts.Token);
            _ = AcceptHttpsConnectionsAsync(_cts.Token);
            Logger.LogInfo("Proxy service started successfully");
        }
        catch (Exception ex)
        {
            Logger.LogError("Failed to start proxy service", ex);
            Stop();
            throw;
        }
    }

    public void Stop()
    {
        Logger.LogInfo("Stopping proxy service");
        _isRunning = false;
        _cts?.Cancel();

        try
        {
            _httpListener?.Stop();
            Logger.LogInfo("HTTP listener stopped");
            _httpsListener?.Stop();
            Logger.LogInfo("HTTPS listener stopped");
        }
        catch (Exception ex)
        {
            Logger.LogError("Error stopping proxy service", ex);
        }
    }

    private async Task AcceptHttpConnectionsAsync(CancellationToken cancellationToken)
    {
        Logger.LogInfo("Starting HTTP connection acceptor");
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = await _httpListener!.AcceptTcpClientAsync(cancellationToken);
                Logger.LogInfo($"HTTP connection accepted from {client.Client.RemoteEndPoint}");
                _ = HandleHttpConnectionAsync(client, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                Logger.LogInfo("HTTP connection acceptor cancelled");
                break;
            }
            catch (Exception ex)
            {
                Logger.LogError("Error accepting HTTP connection", ex);
                break;
            }
        }
    }

    private async Task AcceptHttpsConnectionsAsync(CancellationToken cancellationToken)
    {
        Logger.LogInfo("Starting HTTPS connection acceptor");
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = await _httpsListener!.AcceptTcpClientAsync(cancellationToken);
                Logger.LogInfo($"HTTPS connection accepted from {client.Client.RemoteEndPoint}");
                _ = HandleHttpsConnectionAsync(client, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                Logger.LogInfo("HTTPS connection acceptor cancelled");
                break;
            }
            catch (Exception ex)
            {
                Logger.LogError("Error accepting HTTPS connection", ex);
                break;
            }
        }
    }

    private async Task HandleHttpConnectionAsync(TcpClient client, CancellationToken cancellationToken)
    {
        using var clientStream = client.GetStream();
        var remoteEP = client.Client.RemoteEndPoint;
        
        try
            {
                Logger.LogInfo($"Handling HTTP connection from {remoteEP}");
                
                byte[] requestData = await ReadUntilEnd(clientStream, cancellationToken);
                Logger.LogInfo($"Request data read: {requestData.Length} bytes");
                
                if (requestData.Length == 0)
                {
                    Logger.LogWarning("Empty HTTP request data");
                    return;
                }

                string requestStr = Encoding.UTF8.GetString(requestData);
                Logger.LogInfo($"Request content:\n{requestStr}");
                string[] lines = requestStr.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
                
                if (lines.Length == 0)
                {
                    Logger.LogWarning("No lines in HTTP request");
                    return;
                }

                string firstLine = lines[0];
                string[] parts = firstLine.Split(new[] { ' ' }, 3);
                
                if (parts.Length < 2)
                {
                    Logger.LogWarning($"Invalid HTTP request line: {firstLine}");
                    return;
                }

                string method = parts[0];
                string url = parts[1];
                Logger.LogInfo($"HTTP request: {method} {url}");

                string host = "example.com";
                foreach (string line in lines)
                {
                    if (line.StartsWith("Host:", StringComparison.OrdinalIgnoreCase))
                    {
                        host = line.Substring(5).Trim();
                        break;
                    }
                }
                Logger.LogInfo($"Extracted Host: {host}");

                Uri uri;
                if (!Uri.TryCreate(url, UriKind.Absolute, out uri))
                {
                    uri = new Uri($"http://{host}{url}");
                }

                Logger.LogInfo($"Connecting to remote server: {uri.Host}:{uri.Port}");
                var remoteClient = new TcpClient();
                await remoteClient.ConnectAsync(uri.Host, uri.Port, cancellationToken);
                Logger.LogInfo("Connected to remote server");
                using var remoteStream = remoteClient.GetStream();

                await remoteStream.WriteAsync(requestData, cancellationToken);
                Logger.LogInfo("Request sent to remote server");
                
                byte[] responseData = await ReadUntilEnd(remoteStream, cancellationToken);
                Logger.LogInfo($"Response received: {responseData.Length} bytes");

                await clientStream.WriteAsync(responseData, cancellationToken);
                Logger.LogInfo("Response sent to client");

                var record = _httpParser.ParseRequestAndResponse(requestData, responseData);
                record.Method = method;
                record.Url = url;
                record.IsHttps = false;
                record.Timestamp = DateTime.Now;
                record.RemoteAddress = uri.Host;
                record.RemotePort = uri.Port;

                Logger.LogInfo($"Record captured: {method} {url}");
                _onRecordCaptured?.Invoke(record);
            }
            catch (Exception ex)
            {
                Logger.LogError($"Error handling HTTP connection from {remoteEP}", ex);
            }
    }

    private async Task HandleHttpsConnectionAsync(TcpClient client, CancellationToken cancellationToken)
    {
        using var clientStream = client.GetStream();
        var remoteEP = client.Client.RemoteEndPoint;

        try
        {
            Logger.LogInfo($"Handling HTTPS connection from {remoteEP}");
            
            byte[] clientHello = await ReadClientHello(clientStream, cancellationToken);
            Logger.LogInfo($"ClientHello received: {clientHello.Length} bytes");
            
            if (!_tlsParser.TryParseClientHello(clientHello, out var hello))
            {
                Logger.LogWarning("Failed to parse ClientHello");
                return;
            }

            string sni = hello.Sni;
            Logger.LogInfo($"SNI extracted: {sni}");
            
            if (string.IsNullOrEmpty(sni))
            {
                Logger.LogWarning("Empty SNI");
                return;
            }

            var cert = _certificateCache.GetCertificateForHost(sni);
            Logger.LogInfo($"Certificate obtained for {sni}");

            var sslClientStream = new SslStream(clientStream, false);
            await sslClientStream.AuthenticateAsServerAsync(
                cert,
                false,
                SslProtocols.Tls12 | SslProtocols.Tls13,
                false
            );
            Logger.LogInfo("TLS handshake with client completed");

            var remoteClient = new TcpClient();
            await remoteClient.ConnectAsync(sni, 443, cancellationToken);
            Logger.LogInfo($"Connected to remote server {sni}:443");
            
            var sslServerStream = new SslStream(remoteClient.GetStream(), false);
            await sslServerStream.AuthenticateAsClientAsync(sni);
            Logger.LogInfo("TLS handshake with server completed");

            var requestBuffer = new MemoryStream();
            var responseBuffer = new MemoryStream();

            var relayTask = RelayAsync(sslClientStream, sslServerStream, requestBuffer, cancellationToken);
            var backRelayTask = RelayAsync(sslServerStream, sslClientStream, responseBuffer, cancellationToken);

            await Task.WhenAll(relayTask, backRelayTask);
            Logger.LogInfo($"Relay completed. Request: {requestBuffer.Length} bytes, Response: {responseBuffer.Length} bytes");

            var record = _httpParser.ParseRequestAndResponse(requestBuffer.ToArray(), responseBuffer.ToArray());
            record.Method = "CONNECT";
            record.Url = $"https://{sni}/";
            record.IsHttps = true;
            record.Sni = sni;
            record.Timestamp = DateTime.Now;
            record.RemoteAddress = sni;
            record.RemotePort = 443;

            Logger.LogInfo($"HTTPS record captured: {sni}");
            _onRecordCaptured?.Invoke(record);
        }
        catch (Exception ex)
        {
            Logger.LogError($"Error handling HTTPS connection from {remoteEP}", ex);
        }
    }

    private async Task<byte[]> ReadClientHello(Stream stream, CancellationToken cancellationToken)
    {
        var buffer = new MemoryStream();
        var header = new byte[5];
        int bytesRead = 0;

        while (bytesRead < 5)
        {
            int read = await stream.ReadAsync(header.AsMemory(bytesRead), cancellationToken);
            if (read == 0)
                break;
            bytesRead += read;
        }

        if (bytesRead < 5 || header[0] != 0x16)
            return Array.Empty<byte>();

        int length = (header[3] << 8) | header[4];
        buffer.Write(header, 0, 5);

        byte[] body = new byte[length];
        bytesRead = 0;
        
        while (bytesRead < length)
        {
            int read = await stream.ReadAsync(body.AsMemory(bytesRead), cancellationToken);
            if (read == 0)
                break;
            bytesRead += read;
        }

        buffer.Write(body, 0, bytesRead);
        return buffer.ToArray();
    }

    private async Task<byte[]> ReadUntilEnd(Stream stream, CancellationToken cancellationToken)
    {
        var buffer = new MemoryStream();
        byte[] chunk = new byte[4096];
        int bytesRead;
        int totalBytes = 0;

        stream.ReadTimeout = 5000;
        Logger.LogInfo("Starting ReadUntilEnd");

        byte[] httpEndMarker = Encoding.ASCII.GetBytes("\r\n\r\n");
        bool foundEndMarker = false;

        while (!cancellationToken.IsCancellationRequested && !foundEndMarker)
        {
            try
            {
                bytesRead = await stream.ReadAsync(chunk, cancellationToken);
                Logger.LogInfo($"Read {bytesRead} bytes in ReadUntilEnd");
                if (bytesRead == 0)
                {
                    Logger.LogInfo("Read returned 0 bytes, ending read");
                    break;
                }
                
                buffer.Write(chunk, 0, bytesRead);
                totalBytes += bytesRead;

                byte[] currentBuffer = buffer.ToArray();
                if (currentBuffer.Length >= 4)
                {
                    for (int i = 0; i <= currentBuffer.Length - 4; i++)
                    {
                        if (currentBuffer[i] == httpEndMarker[0] &&
                            currentBuffer[i+1] == httpEndMarker[1] &&
                            currentBuffer[i+2] == httpEndMarker[2] &&
                            currentBuffer[i+3] == httpEndMarker[3])
                        {
                            foundEndMarker = true;
                            Logger.LogInfo("Found HTTP end marker");
                            break;
                        }
                    }
                }
            }
            catch (IOException ex)
            {
                Logger.LogInfo($"IOException in ReadUntilEnd: {ex.Message}");
                break;
            }
            catch (Exception ex)
            {
                Logger.LogError($"Unexpected error in ReadUntilEnd", ex);
                break;
            }
        }

        Logger.LogInfo($"ReadUntilEnd completed, total bytes: {totalBytes}");
        return buffer.ToArray();
    }

    private async Task RelayAsync(Stream source, Stream destination, MemoryStream captureBuffer, CancellationToken cancellationToken)
    {
        byte[] buffer = new byte[4096];
        int bytesRead;

        while (!cancellationToken.IsCancellationRequested)
        {
            bytesRead = await source.ReadAsync(buffer, cancellationToken);
            if (bytesRead == 0)
                break;

            captureBuffer.Write(buffer, 0, bytesRead);
            await destination.WriteAsync(buffer.AsMemory(0, bytesRead), cancellationToken);
        }
    }

    public void Dispose()
    {
        Stop();
    }

    public bool IsRunning => _isRunning;
}