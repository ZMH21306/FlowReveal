using System;
using System.IO;
using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Threading.Tasks;
using FlowReveal.Models;

namespace FlowReveal.Services
{
    public class ProxyService : IProxyService
    {
        private TcpListener _listener;
        private bool _running;
        private readonly ICertificateService _certificateService;
        private int _port;

        public bool IsRunning => _running;

        public event EventHandler<HttpLogEntry> RequestCaptured;

        public ProxyService(ICertificateService certificateService)
        {
            _certificateService = certificateService;
            _port = 8888;
        }

        public ProxyService(ICertificateService certificateService, int port)
        {
            _certificateService = certificateService;
            _port = port;
        }

        public async Task StartAsync()
        {
            _listener = new TcpListener(IPAddress.Loopback, _port);
            _listener.Start();
            _running = true;

            Console.WriteLine($"Proxy server started on port {_port}");

            while (_running)
            {
                try
                {
                    var client = await _listener.AcceptTcpClientAsync();
                    _ = HandleClientAsync(client);
                }
                catch (Exception ex)
                {
                    if (!_running)
                        break;
                    Console.WriteLine($"Proxy error: {ex.Message}");
                }
            }
        }

        public Task StopAsync()
        {
            _running = false;
            _listener?.Stop();
            Console.WriteLine("Proxy server stopped");
            return Task.CompletedTask;
        }

        private async Task HandleClientAsync(TcpClient client)
        {
            using (client)
            {
                try
                {
                    NetworkStream clientStream = client.GetStream();
                    StreamReader reader = new StreamReader(clientStream, Encoding.ASCII, false, 4096, true);

                    // 读取第一行（请求行或 CONNECT 命令）
                    string firstLine = await reader.ReadLineAsync();
                    if (string.IsNullOrEmpty(firstLine))
                        return;

                    Console.WriteLine($"Received request: {firstLine}");

                    if (firstLine.StartsWith("CONNECT", StringComparison.OrdinalIgnoreCase))
                    {
                        // HTTPS CONNECT 请求
                        await HandleHttpsConnectAsync(client, clientStream, reader, firstLine);
                    }
                    else
                    {
                        // HTTP 请求
                        await HandleHttpRequestAsync(client, clientStream, reader, firstLine);
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Client handling error: {ex.Message}");
                }
            }
        }

        private async Task HandleHttpsConnectAsync(TcpClient client, NetworkStream clientStream, StreamReader reader, string connectLine)
        {
            // 解析 CONNECT 目标
            // 格式: CONNECT example.com:443 HTTP/1.1
            string[] parts = connectLine.Split(' ');
            if (parts.Length < 2)
                return;

            string target = parts[1];
            string[] hostPort = target.Split(':');
            string host = hostPort[0];
            int port = hostPort.Length > 1 ? int.Parse(hostPort[1]) : 443;

            try
            {
                // 建立到目标服务器的连接
                using (TcpClient server = new TcpClient())
                {
                    await server.ConnectAsync(host, port);

                    // 向客户端发送 200 响应，表示连接已建立
                    byte[] response = Encoding.ASCII.GetBytes("HTTP/1.1 200 Connection Established\r\n\r\n");
                    await clientStream.WriteAsync(response, 0, response.Length);
                    await clientStream.FlushAsync();

                    // 生成目标域名的证书
                    X509Certificate2 cert = await _certificateService.CreateDomainCertificateAsync(host);

                    // 使用证书与客户端建立 TLS 连接
                    SslStream clientSslStream = new SslStream(clientStream, false);
                    await clientSslStream.AuthenticateAsServerAsync(cert, false, System.Security.Authentication.SslProtocols.Tls12 | System.Security.Authentication.SslProtocols.Tls13, false);

                    // 与服务器建立 TLS 连接
                    SslStream serverSslStream = new SslStream(server.GetStream(), false);
                    await serverSslStream.AuthenticateAsClientAsync(host);

                    // 双向数据转发
                    await Task.WhenAll(
                        ForwardStreamAsync(clientSslStream, serverSslStream, true),
                        ForwardStreamAsync(serverSslStream, clientSslStream, false)
                    );
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"HTTPS error: {ex.Message}");
                // 发送错误响应
                byte[] errorResponse = Encoding.ASCII.GetBytes("HTTP/1.1 500 Internal Server Error\r\n\r\n");
                await clientStream.WriteAsync(errorResponse, 0, errorResponse.Length);
            }
        }

        private async Task HandleHttpRequestAsync(TcpClient client, NetworkStream clientStream, StreamReader reader, string firstLine)
        {
            try
            {
                // 解析请求行
                string[] requestParts = firstLine.Split(' ');
                string method = requestParts[0];
                string url = requestParts[1];
                string httpVersion = requestParts[2];

                // 解析 URL 获取主机和路径
                Uri uri;
                if (url.StartsWith("http://"))
                {
                    uri = new Uri(url);
                }
                else
                {
                    // 相对路径，需要从 Host 头获取主机
                    uri = new Uri("http://example.com" + url);
                }

                // 读取请求头
                StringBuilder headers = new StringBuilder();
                string line;
                while ((line = await reader.ReadLineAsync()) != null && line != string.Empty)
                {
                    headers.AppendLine(line);
                }
                string requestHeaders = headers.ToString();

                // 建立到目标服务器的连接
                using (TcpClient server = new TcpClient())
                {
                    await server.ConnectAsync(uri.Host, uri.Port == 0 ? 80 : uri.Port);
                    NetworkStream serverStream = server.GetStream();

                    // 转发请求到服务器
                    string fullRequest = $"{method} {uri.PathAndQuery} {httpVersion}\r\n{requestHeaders}\r\n";
                    byte[] requestBytes = Encoding.ASCII.GetBytes(fullRequest);
                    await serverStream.WriteAsync(requestBytes, 0, requestBytes.Length);
                    await serverStream.FlushAsync();

                    // 读取服务器响应
                    StreamReader serverReader = new StreamReader(serverStream, Encoding.ASCII, false, 4096, true);
                    string statusLine = await serverReader.ReadLineAsync();

                    StringBuilder responseHeaders = new StringBuilder();
                    while ((line = await serverReader.ReadLineAsync()) != null && line != string.Empty)
                    {
                        responseHeaders.AppendLine(line);
                    }

                    // 读取响应体
                    string responseBody = await serverReader.ReadToEndAsync();

                    // 转发响应给客户端
                    string fullResponse = $"{statusLine}\r\n{responseHeaders}\r\n{responseBody}";
                    byte[] responseBytes = Encoding.ASCII.GetBytes(fullResponse);
                    await clientStream.WriteAsync(responseBytes, 0, responseBytes.Length);
                    await clientStream.FlushAsync();

                    // 创建日志条目
                    HttpLogEntry entry = new HttpLogEntry
                    {
                        Timestamp = DateTime.Now,
                        Method = method,
                        Url = url,
                        RequestHeaders = requestHeaders,
                        ResponseHeaders = responseHeaders.ToString(),
                        RequestBody = string.Empty, // POST 请求体需要单独处理
                        ResponseBody = responseBody,
                        Host = uri.Host,
                        Scheme = "http",
                        IsHttps = false,
                        StatusCode = ParseStatusCode(statusLine)
                    };

                    RequestCaptured?.Invoke(this, entry);
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"HTTP error: {ex.Message}");
            }
        }

        private async Task ForwardStreamAsync(Stream source, Stream destination, bool isClientToServer)
        {
            byte[] buffer = new byte[8192];
            int bytesRead;
            while ((bytesRead = await source.ReadAsync(buffer, 0, buffer.Length)) > 0)
            {
                await destination.WriteAsync(buffer, 0, bytesRead);
                await destination.FlushAsync();

                // 如果是客户端到服务器的数据，可以在这里解析 HTTP 协议
                if (isClientToServer)
                {
                    string data = Encoding.ASCII.GetString(buffer, 0, bytesRead);
                    Console.WriteLine($"Client -> Server ({bytesRead} bytes): {data.Substring(0, Math.Min(100, data.Length))}...");
                }
            }
        }

        private int ParseStatusCode(string statusLine)
        {
            if (string.IsNullOrEmpty(statusLine))
                return 0;

            string[] parts = statusLine.Split(' ');
            if (parts.Length >= 2 && int.TryParse(parts[1], out int code))
                return code;

            return 0;
        }
    }
}
