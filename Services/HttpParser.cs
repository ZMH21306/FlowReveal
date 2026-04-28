using FlowReveal.Models;
using System;
using System.IO;
using System.Text;
using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public class HttpParser : IHttpParser
    {
        public async Task<HttpLogEntry> ParseHttpRequestAsync(Stream stream)
        {
            var entry = new HttpLogEntry
            {
                Timestamp = DateTime.Now,
                IsHttps = false
            };

            using (var reader = new StreamReader(stream, Encoding.ASCII, false, 1024, true))
            {
                // 解析请求行
                var requestLine = await reader.ReadLineAsync();
                if (requestLine == null)
                    return entry;

                var parts = requestLine.Split(' ');
                if (parts.Length >= 3)
                {
                    entry.Method = parts[0];
                    entry.Url = parts[1];
                }

                // 解析请求头
                var headers = new StringBuilder();
                string line;
                while ((line = await reader.ReadLineAsync()) != null && line != string.Empty)
                {
                    headers.AppendLine(line);
                }
                entry.RequestHeaders = headers.ToString();

                // 解析请求体
                // 这里需要根据 Content-Length 或 Transfer-Encoding 来读取请求体
                // 暂时简化实现
            }

            return entry;
        }

        public async Task<HttpLogEntry> ParseHttpResponseAsync(Stream stream, HttpLogEntry requestEntry)
        {
            var entry = requestEntry;

            using (var reader = new StreamReader(stream, Encoding.ASCII, false, 1024, true))
            {
                // 解析状态行
                var statusLine = await reader.ReadLineAsync();
                if (statusLine == null)
                    return entry;

                var parts = statusLine.Split(' ');
                if (parts.Length >= 3)
                {
                    int statusCode;
                    int.TryParse(parts[1], out statusCode);
                    entry.StatusCode = statusCode;
                }

                // 解析响应头
                var headers = new StringBuilder();
                string line;
                while ((line = await reader.ReadLineAsync()) != null && line != string.Empty)
                {
                    headers.AppendLine(line);
                }
                entry.ResponseHeaders = headers.ToString();

                // 解析响应体
                // 这里需要根据 Content-Length 或 Transfer-Encoding 来读取响应体
                // 暂时简化实现
            }

            entry.ResponseTimeMs = (DateTime.Now - entry.Timestamp).Milliseconds;
            return entry;
        }
    }
}
