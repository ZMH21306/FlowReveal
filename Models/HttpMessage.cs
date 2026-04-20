using System.Collections.Generic;

namespace FlowReveal.Models
{
    public class HttpMessage
    {
        public string Method { get; set; }
        public string Url { get; set; }
        public string HttpVersion { get; set; }
        public int StatusCode { get; set; }
        public string StatusMessage { get; set; }
        public Dictionary<string, string> Headers { get; set; }
        public Dictionary<string, string> Cookies { get; set; }
        public Dictionary<string, string> QueryParameters { get; set; }
        public string Body { get; set; }
        public long BodySize { get; set; }
        public string ContentType { get; set; }
        public string Host { get; set; }
        public string UserAgent { get; set; }
        public string Referer { get; set; }
        public string Accept { get; set; }
        public string AcceptEncoding { get; set; }
        public string AcceptLanguage { get; set; }
        public string Connection { get; set; }
        public string CacheControl { get; set; }
        public string Authorization { get; set; }
        public bool IsHttps { get; set; }
        public bool IsRequest { get; set; }
        public string SessionId { get; set; }

        public HttpMessage()
        {
            Headers = new Dictionary<string, string>();
            Cookies = new Dictionary<string, string>();
            QueryParameters = new Dictionary<string, string>();
        }
    }
}