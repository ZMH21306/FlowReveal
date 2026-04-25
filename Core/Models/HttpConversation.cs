using System;
using System.Collections.Generic;

namespace FlowReveal.Core.Models
{
    public class HttpRequest
    {
        public string Method { get; set; } = string.Empty;
        public string Url { get; set; } = string.Empty;
        public string Path { get; set; } = string.Empty;
        public string QueryString { get; set; } = string.Empty;
        public string HttpVersion { get; set; } = "HTTP/1.1";
        public Dictionary<string, string> Headers { get; set; } = new();
        public byte[] Body { get; set; } = Array.Empty<byte>();
        public string ContentType => Headers.GetValueOrDefault("Content-Type", string.Empty);
        public long ContentLength => Headers.TryGetValue("Content-Length", out var val) && long.TryParse(val, out var len) ? len : Body.Length;
    }

    public class HttpResponse
    {
        public string HttpVersion { get; set; } = "HTTP/1.1";
        public int StatusCode { get; set; }
        public string StatusDescription { get; set; } = string.Empty;
        public Dictionary<string, string> Headers { get; set; } = new();
        public byte[] Body { get; set; } = Array.Empty<byte>();
        public string ContentType => Headers.GetValueOrDefault("Content-Type", string.Empty);
        public long ContentLength => Headers.TryGetValue("Content-Length", out var val) && long.TryParse(val, out var len) ? len : Body.Length;
    }

    public class HttpConversation
    {
        public Guid Id { get; } = Guid.NewGuid();
        public DateTime StartTime { get; set; } = DateTime.UtcNow;
        public DateTime EndTime { get; set; }
        public HttpRequest Request { get; set; } = new();
        public HttpResponse Response { get; set; } = new();
        public TimeSpan Duration => EndTime > StartTime ? EndTime - StartTime : TimeSpan.Zero;
        public bool HasResponse => Response.StatusCode != 0;
        public bool IsHttps { get; set; }
        public string Host => Request.Headers.GetValueOrDefault("Host", Request.Url);
        public long TotalSize => Request.Body.Length + Response.Body.Length;
        public bool IsError => Response.StatusCode >= 400;
        public bool IsSlow => Duration.TotalSeconds > 3;
    }
}
