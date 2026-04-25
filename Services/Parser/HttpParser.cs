using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Text;

namespace FlowReveal.Services.Parser
{
    public class ParsedHttpRequest
    {
        public string Method { get; set; } = string.Empty;
        public string Url { get; set; } = string.Empty;
        public string Path { get; set; } = string.Empty;
        public string QueryString { get; set; } = string.Empty;
        public string HttpVersion { get; set; } = "HTTP/1.1";
        public Dictionary<string, string> Headers { get; set; } = new(StringComparer.OrdinalIgnoreCase);
        public byte[] Body { get; set; } = Array.Empty<byte>();
    }

    public class ParsedHttpResponse
    {
        public string HttpVersion { get; set; } = "HTTP/1.1";
        public int StatusCode { get; set; }
        public string StatusDescription { get; set; } = string.Empty;
        public Dictionary<string, string> Headers { get; set; } = new(StringComparer.OrdinalIgnoreCase);
        public byte[] Body { get; set; } = Array.Empty<byte>();
    }

    public static class HttpParser
    {
        private static readonly byte[] HeaderDelimiter = Encoding.ASCII.GetBytes("\r\n\r\n");
        private static readonly byte[] LineDelimiter = Encoding.ASCII.GetBytes("\r\n");

        public static bool TryParseRequest(byte[] data, int offset, int length, out ParsedHttpRequest? request, out int consumedBytes)
        {
            request = null;
            consumedBytes = 0;

            var headerEnd = IndexOf(data, offset, length, HeaderDelimiter);
            if (headerEnd < 0)
                return false;

            var headerLength = headerEnd - offset;
            var headerText = Encoding.ASCII.GetString(data, offset, headerLength);
            var lines = headerText.Split("\r\n", StringSplitOptions.RemoveEmptyEntries);

            if (lines.Length == 0)
                return false;

            var requestLine = lines[0];
            var parts = requestLine.Split(' ', 3);
            if (parts.Length < 2)
                return false;

            var result = new ParsedHttpRequest
            {
                Method = parts[0],
                Url = parts.Length > 1 ? parts[1] : string.Empty,
                HttpVersion = parts.Length > 2 ? parts[2] : "HTTP/1.1"
            };

            var uri = result.Url;
            var queryIndex = uri.IndexOf('?');
            if (queryIndex >= 0)
            {
                result.Path = uri.Substring(0, queryIndex);
                result.QueryString = uri.Substring(queryIndex + 1);
            }
            else
            {
                result.Path = uri;
            }

            for (int i = 1; i < lines.Length; i++)
            {
                var colonIndex = lines[i].IndexOf(':');
                if (colonIndex > 0)
                {
                    var key = lines[i].Substring(0, colonIndex).Trim();
                    var value = lines[i].Substring(colonIndex + 1).Trim();
                    result.Headers[key] = value;
                }
            }

            var bodyOffset = headerEnd + HeaderDelimiter.Length;
            var bodyLength = length - (bodyOffset - offset);

            if (result.Headers.TryGetValue("Content-Length", out var contentLengthStr) &&
                int.TryParse(contentLengthStr, out var contentLength))
            {
                if (bodyLength < contentLength)
                    return false;

                result.Body = new byte[contentLength];
                Array.Copy(data, bodyOffset, result.Body, 0, contentLength);
                consumedBytes = (bodyOffset - offset) + contentLength;
            }
            else if (result.Headers.TryGetValue("Transfer-Encoding", out var transferEncoding) &&
                     transferEncoding.Contains("chunked", StringComparison.OrdinalIgnoreCase))
            {
                if (!TryParseChunkedBody(data, bodyOffset, bodyLength, out var body, out var chunkedConsumed))
                    return false;

                result.Body = body;
                consumedBytes = (bodyOffset - offset) + chunkedConsumed;
            }
            else
            {
                consumedBytes = bodyOffset - offset;
                result.Body = Array.Empty<byte>();
            }

            request = result;
            return true;
        }

        public static bool TryParseResponse(byte[] data, int offset, int length, out ParsedHttpResponse? response, out int consumedBytes)
        {
            response = null;
            consumedBytes = 0;

            var headerEnd = IndexOf(data, offset, length, HeaderDelimiter);
            if (headerEnd < 0)
                return false;

            var headerLength = headerEnd - offset;
            var headerText = Encoding.ASCII.GetString(data, offset, headerLength);
            var lines = headerText.Split("\r\n", StringSplitOptions.RemoveEmptyEntries);

            if (lines.Length == 0)
                return false;

            var statusLine = lines[0];
            var parts = statusLine.Split(' ', 3);
            if (parts.Length < 2)
                return false;

            var result = new ParsedHttpResponse
            {
                HttpVersion = parts[0],
                StatusCode = int.TryParse(parts.Length > 1 ? parts[1] : "0", out var code) ? code : 0,
                StatusDescription = parts.Length > 2 ? parts[2] : string.Empty
            };

            for (int i = 1; i < lines.Length; i++)
            {
                var colonIndex = lines[i].IndexOf(':');
                if (colonIndex > 0)
                {
                    var key = lines[i].Substring(0, colonIndex).Trim();
                    var value = lines[i].Substring(colonIndex + 1).Trim();
                    result.Headers[key] = value;
                }
            }

            var bodyOffset = headerEnd + HeaderDelimiter.Length;
            var bodyLength = length - (bodyOffset - offset);

            if (result.Headers.TryGetValue("Content-Length", out var contentLengthStr) &&
                int.TryParse(contentLengthStr, out var contentLength))
            {
                if (bodyLength < contentLength)
                    return false;

                result.Body = new byte[contentLength];
                Array.Copy(data, bodyOffset, result.Body, 0, contentLength);
                consumedBytes = (bodyOffset - offset) + contentLength;
            }
            else if (result.Headers.TryGetValue("Transfer-Encoding", out var transferEncoding) &&
                     transferEncoding.Contains("chunked", StringComparison.OrdinalIgnoreCase))
            {
                if (!TryParseChunkedBody(data, bodyOffset, bodyLength, out var body, out var chunkedConsumed))
                    return false;

                result.Body = body;
                consumedBytes = (bodyOffset - offset) + chunkedConsumed;
            }
            else if (result.StatusCode >= 100 && result.StatusCode < 200 ||
                     result.StatusCode == 204 || result.StatusCode == 304)
            {
                consumedBytes = bodyOffset - offset;
                result.Body = Array.Empty<byte>();
            }
            else
            {
                consumedBytes = bodyOffset - offset;
                result.Body = bodyLength > 0 ? data[bodyOffset..(offset + length)] : Array.Empty<byte>();
            }

            response = result;
            return true;
        }

        public static byte[] DecodeContent(byte[] body, string? contentEncoding)
        {
            if (string.IsNullOrEmpty(contentEncoding))
                return body;

            try
            {
                if (contentEncoding.Contains("gzip", StringComparison.OrdinalIgnoreCase))
                {
                    using var ms = new MemoryStream(body);
                    using var gzip = new GZipStream(ms, CompressionMode.Decompress);
                    using var output = new MemoryStream();
                    gzip.CopyTo(output);
                    return output.ToArray();
                }

                if (contentEncoding.Contains("deflate", StringComparison.OrdinalIgnoreCase))
                {
                    using var ms = new MemoryStream(body);
                    using var deflate = new DeflateStream(ms, CompressionMode.Decompress);
                    using var output = new MemoryStream();
                    deflate.CopyTo(output);
                    return output.ToArray();
                }
            }
            catch
            {
                return body;
            }

            return body;
        }

        private static bool TryParseChunkedBody(byte[] data, int offset, int availableLength, out byte[] body, out int consumedBytes)
        {
            body = Array.Empty<byte>();
            consumedBytes = 0;

            var bodyParts = new List<byte[]>();
            var totalBodyLength = 0;
            var currentOffset = offset;

            while (currentOffset < offset + availableLength)
            {
                var lineEnd = IndexOf(data, currentOffset, offset + availableLength - currentOffset, LineDelimiter);
                if (lineEnd < 0)
                    return false;

                var chunkSizeText = Encoding.ASCII.GetString(data, currentOffset, lineEnd - currentOffset).Trim();
                var semiColonIndex = chunkSizeText.IndexOf(';');
                if (semiColonIndex >= 0)
                    chunkSizeText = chunkSizeText.Substring(0, semiColonIndex);

                if (!int.TryParse(chunkSizeText, System.Globalization.NumberStyles.HexNumber, null, out var chunkSize))
                    return false;

                currentOffset = lineEnd + LineDelimiter.Length;

                if (chunkSize == 0)
                {
                    if (currentOffset + LineDelimiter.Length <= offset + availableLength)
                        currentOffset += LineDelimiter.Length;

                    consumedBytes = currentOffset - offset;

                    var result = new byte[totalBodyLength];
                    var destOffset = 0;
                    foreach (var part in bodyParts)
                    {
                        Array.Copy(part, 0, result, destOffset, part.Length);
                        destOffset += part.Length;
                    }

                    body = result;
                    return true;
                }

                if (currentOffset + chunkSize + LineDelimiter.Length > offset + availableLength)
                    return false;

                var chunkData = new byte[chunkSize];
                Array.Copy(data, currentOffset, chunkData, 0, chunkSize);
                bodyParts.Add(chunkData);
                totalBodyLength += chunkSize;

                currentOffset += chunkSize + LineDelimiter.Length;
            }

            return false;
        }

        public static bool LooksLikeHttpRequest(byte[] data, int offset, int length)
        {
            if (length < 4) return false;

            var methods = new[] { "GET ", "POST", "PUT ", "DELE", "PATC", "HEAD", "OPTI", "TRAC", "CONN" };
            var prefix = Encoding.ASCII.GetString(data, offset, Math.Min(4, length));

            foreach (var method in methods)
            {
                if (prefix == method) return true;
            }

            return false;
        }

        public static bool LooksLikeHttpResponse(byte[] data, int offset, int length)
        {
            if (length < 5) return false;
            var prefix = Encoding.ASCII.GetString(data, offset, 5);
            return prefix == "HTTP/";
        }

        private static int IndexOf(byte[] data, int offset, int length, byte[] pattern)
        {
            for (int i = offset; i <= offset + length - pattern.Length; i++)
            {
                bool found = true;
                for (int j = 0; j < pattern.Length; j++)
                {
                    if (data[i + j] != pattern[j])
                    {
                        found = false;
                        break;
                    }
                }
                if (found) return i;
            }
            return -1;
        }
    }
}
