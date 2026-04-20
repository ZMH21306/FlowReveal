using System;
using System.Collections.Generic;
using FlowReveal.Models;

namespace FlowReveal.Core.Session
{
    public class HttpSession
    {
        public string SessionId { get; private set; }
        public DateTime StartTime { get; private set; }
        public DateTime? EndTime { get; private set; }
        public HttpMessage Request { get; set; }
        public HttpMessage Response { get; set; }
        public List<HttpMessage> AllMessages { get; private set; }
        public long TotalBytesSent { get; set; }
        public long TotalBytesReceived { get; set; }
        public string SourceIp { get; set; }
        public int SourcePort { get; set; }
        public string DestinationIp { get; set; }
        public int DestinationPort { get; set; }
        public bool IsHttps { get; set; }
        public string Hostname { get; set; }
        public string Path { get; set; }

        public HttpSession()
        {
            SessionId = Guid.NewGuid().ToString();
            StartTime = DateTime.Now;
            AllMessages = new List<HttpMessage>();
        }

        public void AddMessage(HttpMessage message)
        {
            AllMessages.Add(message);
            message.SessionId = SessionId;

            if (message.IsRequest)
            {
                Request = message;
                if (message.Url != null)
                {
                    // 提取路径
                    if (message.Url.StartsWith("http"))
                    {
                        var uri = new Uri(message.Url);
                        Hostname = uri.Host;
                        Path = uri.PathAndQuery;
                        IsHttps = uri.Scheme == "https";
                    }
                    else
                    {
                        Path = message.Url;
                    }
                }
            }
            else
            {
                Response = message;
                EndTime = DateTime.Now;
            }
        }

        public bool IsComplete => Request != null && Response != null;

        public TimeSpan Duration => EndTime.HasValue ? EndTime.Value - StartTime : TimeSpan.Zero;

        public void Close()
        {
            if (!EndTime.HasValue)
            {
                EndTime = DateTime.Now;
            }
        }
    }
}