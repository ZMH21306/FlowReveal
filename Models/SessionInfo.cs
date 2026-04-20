using System;using System.Collections.Generic;

namespace FlowReveal.Models
{
    public class SessionInfo
    {
        public string SessionId { get; set; }
        public DateTime StartTime { get; set; }
        public DateTime? EndTime { get; set; }
        public string SourceIp { get; set; }
        public int SourcePort { get; set; }
        public string DestinationIp { get; set; }
        public int DestinationPort { get; set; }
        public bool IsHttps { get; set; }
        public HttpMessage Request { get; set; }
        public HttpMessage Response { get; set; }
        public List<HttpMessage> AllMessages { get; set; }
        public long TotalBytesSent { get; set; }
        public long TotalBytesReceived { get; set; }
        public string Hostname { get; set; }
        public string Path { get; set; }

        public SessionInfo()
        {
            SessionId = Guid.NewGuid().ToString();
            StartTime = DateTime.Now;
            AllMessages = new List<HttpMessage>();
        }
    }
}