using System.IO.Compression;
using System.Text;
using FlowReveal.Services.Parser;

namespace FlowReveal.Tests;

public class HttpParserTests
{
    private static byte[] BuildHttpRequestBytes(string method, string url, string? host = null, string? contentLength = null, byte[]? body = null, string? transferEncoding = null)
    {
        var sb = new StringBuilder();
        sb.Append($"{method} {url} HTTP/1.1\r\n");
        if (host != null)
            sb.Append($"Host: {host}\r\n");
        if (contentLength != null)
            sb.Append($"Content-Length: {contentLength}\r\n");
        if (transferEncoding != null)
            sb.Append($"Transfer-Encoding: {transferEncoding}\r\n");
        sb.Append("\r\n");

        var headerBytes = Encoding.ASCII.GetBytes(sb.ToString());
        if (body != null)
        {
            var result = new byte[headerBytes.Length + body.Length];
            Array.Copy(headerBytes, result, headerBytes.Length);
            Array.Copy(body, 0, result, headerBytes.Length, body.Length);
            return result;
        }

        return headerBytes;
    }

    private static byte[] BuildHttpResponseBytes(int statusCode, string statusDescription, string? contentLength = null, byte[]? body = null, string? transferEncoding = null, string? contentType = null)
    {
        var sb = new StringBuilder();
        sb.Append($"HTTP/1.1 {statusCode} {statusDescription}\r\n");
        if (contentType != null)
            sb.Append($"Content-Type: {contentType}\r\n");
        if (contentLength != null)
            sb.Append($"Content-Length: {contentLength}\r\n");
        if (transferEncoding != null)
            sb.Append($"Transfer-Encoding: {transferEncoding}\r\n");
        sb.Append("\r\n");

        var headerBytes = Encoding.ASCII.GetBytes(sb.ToString());
        if (body != null)
        {
            var result = new byte[headerBytes.Length + body.Length];
            Array.Copy(headerBytes, result, headerBytes.Length);
            Array.Copy(body, 0, result, headerBytes.Length, body.Length);
            return result;
        }

        return headerBytes;
    }

    private static byte[] BuildChunkedBody(params (int size, byte[] data)[] chunks)
    {
        using var ms = new MemoryStream();
        foreach (var (size, data) in chunks)
        {
            var sizeLine = Encoding.ASCII.GetBytes(size.ToString("X") + "\r\n");
            ms.Write(sizeLine, 0, sizeLine.Length);
            ms.Write(data, 0, data.Length);
            ms.Write(Encoding.ASCII.GetBytes("\r\n"), 0, 2);
        }

        var terminator = Encoding.ASCII.GetBytes("0\r\n\r\n");
        ms.Write(terminator, 0, terminator.Length);

        return ms.ToArray();
    }

    [Fact]
    public void TryParseRequest_GetRequest_ParsesMethodUrlAndHeaders()
    {
        var data = BuildHttpRequestBytes("GET", "/api/test?foo=bar", host: "example.com");

        var result = HttpParser.TryParseRequest(data, 0, data.Length, out var request, out var consumed);

        Assert.True(result);
        Assert.NotNull(request);
        Assert.Equal("GET", request.Method);
        Assert.Equal("/api/test?foo=bar", request.Url);
        Assert.Equal("/api/test", request.Path);
        Assert.Equal("foo=bar", request.QueryString);
        Assert.Equal("example.com", request.Headers["Host"]);
        Assert.Empty(request.Body);
    }

    [Fact]
    public void TryParseRequest_PostWithContentLength_ParsesBody()
    {
        var body = Encoding.UTF8.GetBytes("hello=world");
        var data = BuildHttpRequestBytes("POST", "/submit", host: "example.com", contentLength: body.Length.ToString(), body: body);

        var result = HttpParser.TryParseRequest(data, 0, data.Length, out var request, out var consumed);

        Assert.True(result);
        Assert.NotNull(request);
        Assert.Equal("POST", request.Method);
        Assert.Equal(body, request.Body);
    }

    [Fact]
    public void TryParseResponse_WithStatusCodeAndHeaders_ParsesCorrectly()
    {
        var body = Encoding.UTF8.GetBytes("{\"status\":\"ok\"}");
        var data = BuildHttpResponseBytes(200, "OK", contentLength: body.Length.ToString(), body: body, contentType: "application/json");

        var result = HttpParser.TryParseResponse(data, 0, data.Length, out var response, out var consumed);

        Assert.True(result);
        Assert.NotNull(response);
        Assert.Equal(200, response.StatusCode);
        Assert.Equal("OK", response.StatusDescription);
        Assert.Equal("application/json", response.Headers["Content-Type"]);
        Assert.Equal(body, response.Body);
    }

    [Fact]
    public void TryParseRequest_ChunkedTransferEncoding_ParsesBody()
    {
        var chunk1 = Encoding.UTF8.GetBytes("Hello ");
        var chunk2 = Encoding.UTF8.GetBytes("World");
        var chunkedBody = BuildChunkedBody((chunk1.Length, chunk1), (chunk2.Length, chunk2));
        var data = BuildHttpRequestBytes("POST", "/upload", host: "example.com", transferEncoding: "chunked", body: chunkedBody);

        var result = HttpParser.TryParseRequest(data, 0, data.Length, out var request, out var consumed);

        Assert.True(result);
        Assert.NotNull(request);
        Assert.Equal("Hello World", Encoding.UTF8.GetString(request.Body));
    }

    [Fact]
    public void DecodeContent_GzipContent_DecodesCorrectly()
    {
        var original = Encoding.UTF8.GetBytes("This is gzip compressed content for testing");
        using var compressedMs = new MemoryStream();
        using (var gzip = new GZipStream(compressedMs, CompressionMode.Compress, leaveOpen: true))
        {
            gzip.Write(original, 0, original.Length);
        }

        var compressed = compressedMs.ToArray();
        var decoded = HttpParser.DecodeContent(compressed, "gzip");

        Assert.Equal(original, decoded);
    }

    [Fact]
    public void LooksLikeHttpRequest_ValidMethods_ReturnsTrue()
    {
        var getData = Encoding.ASCII.GetBytes("GET /api HTTP/1.1\r\n");
        var postData = Encoding.ASCII.GetBytes("POST /api HTTP/1.1\r\n");

        Assert.True(HttpParser.LooksLikeHttpRequest(getData, 0, getData.Length));
        Assert.True(HttpParser.LooksLikeHttpRequest(postData, 0, postData.Length));
    }

    [Fact]
    public void LooksLikeHttpRequest_InvalidData_ReturnsFalse()
    {
        var data = Encoding.ASCII.GetBytes("HTTP/1.1 200 OK\r\n");

        Assert.False(HttpParser.LooksLikeHttpRequest(data, 0, data.Length));
    }

    [Fact]
    public void LooksLikeHttpResponse_ValidResponse_ReturnsTrue()
    {
        var data = Encoding.ASCII.GetBytes("HTTP/1.1 200 OK\r\n");

        Assert.True(HttpParser.LooksLikeHttpResponse(data, 0, data.Length));
    }

    [Fact]
    public void LooksLikeHttpResponse_InvalidData_ReturnsFalse()
    {
        var data = Encoding.ASCII.GetBytes("GET /api HTTP/1.1\r\n");

        Assert.False(HttpParser.LooksLikeHttpResponse(data, 0, data.Length));
    }
}
