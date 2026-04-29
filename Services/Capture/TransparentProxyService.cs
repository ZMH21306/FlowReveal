using FlowReveal.Models;
using FlowReveal.Services.Certificate;
using FlowReveal.Services.Http;
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
    }

    public event Action<HttpTrafficRecord>? RecordCaptured
    {
        add => _onRecordCaptured += value;
        remove => _onRecordCaptured -= value;
    }

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        if (_isRunning)
            return;

        _cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        _isRunning = true;

        try
        {
            _httpListener = new TcpListener(IPAddress.Loopback, _httpPort);
            _httpListener.Start();
            
            _httpsListener = new TcpListener(IPAddress.Loopback, _httpsPort);
            _httpsListener.Start();

            _ = AcceptHttpConnectionsAsync(_cts.Token);
            _ = AcceptHttpsConnectionsAsync(_cts.Token);
        }
        catch
        {
            Stop();
            throw;
        }
    }

    public void Stop()
    {
        _isRunning = false;
        _cts?.Cancel();

        try
        {
            _httpListener?.Stop();
            _httpsListener?.Stop();
        }
        catch { }
    }

    private async Task AcceptHttpConnectionsAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = await _httpListener!.AcceptTcpClientAsync(cancellationToken);
                _ = HandleHttpConnectionAsync(client, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch
            {
                break;
            }
        }
    }

    private async Task AcceptHttpsConnectionsAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = await _httpsListener!.AcceptTcpClientAsync(cancellationToken);
                _ = HandleHttpsConnectionAsync(client, cancellationToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch
            {
                break;
            }
        }
    }

    private async Task HandleHttpConnectionAsync(TcpClient client, CancellationToken cancellationToken)
    {
        using var clientStream = client.GetStream();
        
        try
        {
            byte[] requestData = await ReadUntilEnd(clientStream, cancellationToken);
            
            if (requestData.Length == 0)
                return;

            string requestStr = Encoding.UTF8.GetString(requestData);
            string[] lines = requestStr.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
            
            if (lines.Length == 0)
                return;

            string firstLine = lines[0];
            string[] parts = firstLine.Split(new[] { ' ' }, 3);
            
            if (parts.Length < 2)
                return;

            string method = parts[0];
            string url = parts[1];

            Uri uri;
            if (!Uri.TryCreate(url, UriKind.Absolute, out uri))
            {
                uri = new Uri("http://example.com" + url);
            }

            var remoteClient = new TcpClient();
            await remoteClient.ConnectAsync(uri.Host, uri.Port, cancellationToken);
            using var remoteStream = remoteClient.GetStream();

            await remoteStream.WriteAsync(requestData, cancellationToken);
            
            byte[] responseData = await ReadUntilEnd(remoteStream, cancellationToken);

            await clientStream.WriteAsync(responseData, cancellationToken);

            var record = _httpParser.ParseRequestAndResponse(requestData, responseData);
            record.Method = method;
            record.Url = url;
            record.IsHttps = false;
            record.Timestamp = DateTime.Now;
            record.RemoteAddress = uri.Host;
            record.RemotePort = uri.Port;

            _onRecordCaptured?.Invoke(record);
        }
        catch { }
    }

    private async Task HandleHttpsConnectionAsync(TcpClient client, CancellationToken cancellationToken)
    {
        using var clientStream = client.GetStream();

        try
        {
            byte[] clientHello = await ReadClientHello(clientStream, cancellationToken);
            
            if (!_tlsParser.TryParseClientHello(clientHello, out var hello))
                return;

            string sni = hello.Sni;
            
            if (string.IsNullOrEmpty(sni))
                return;

            var cert = _certificateCache.GetCertificateForHost(sni);

            var sslClientStream = new SslStream(clientStream, false);
            await sslClientStream.AuthenticateAsServerAsync(
                cert,
                false,
                SslProtocols.Tls12 | SslProtocols.Tls13,
                false
            );

            var remoteClient = new TcpClient();
            await remoteClient.ConnectAsync(sni, 443, cancellationToken);
            
            var sslServerStream = new SslStream(remoteClient.GetStream(), false);
            await sslServerStream.AuthenticateAsClientAsync(sni);

            var requestBuffer = new MemoryStream();
            var responseBuffer = new MemoryStream();

            var relayTask = RelayAsync(sslClientStream, sslServerStream, requestBuffer, cancellationToken);
            var backRelayTask = RelayAsync(sslServerStream, sslClientStream, responseBuffer, cancellationToken);

            await Task.WhenAll(relayTask, backRelayTask);

            var record = _httpParser.ParseRequestAndResponse(requestBuffer.ToArray(), responseBuffer.ToArray());
            record.Method = "CONNECT";
            record.Url = $"https://{sni}/";
            record.IsHttps = true;
            record.Sni = sni;
            record.Timestamp = DateTime.Now;
            record.RemoteAddress = sni;
            record.RemotePort = 443;

            _onRecordCaptured?.Invoke(record);
        }
        catch { }
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

        stream.ReadTimeout = 5000;

        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                bytesRead = await stream.ReadAsync(chunk, cancellationToken);
                if (bytesRead == 0)
                    break;
                buffer.Write(chunk, 0, bytesRead);
            }
            catch (IOException)
            {
                break;
            }
        }

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