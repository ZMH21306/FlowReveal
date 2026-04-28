using System;

namespace FlowReveal.Models
{
    public class HttpLogEntry
    {
        public int Id { get; set; }
        public DateTime Timestamp { get; set; }
        public string Method { get; set; }
        public string Url { get; set; }
        public int StatusCode { get; set; }
        public long ResponseTimeMs { get; set; }
        public long RequestSize { get; set; }
        public long ResponseSize { get; set; }
        public string ContentType { get; set; }
        public string Host { get; set; }
        public string Scheme { get; set; }
        public string RequestHeaders { get; set; }
        public string ResponseHeaders { get; set; }
        public string RequestBody { get; set; }
        public string ResponseBody { get; set; }
        public bool IsHttps { get; set; }
        public int ProcessId { get; set; }
        public string ProcessName { get; set; }
    }
}
