using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.CompilerServices;

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

    public class HttpConversation : INotifyPropertyChanged
    {
        public Guid Id { get; } = Guid.NewGuid();

        private DateTime _startTime = DateTime.UtcNow;
        public DateTime StartTime
        {
            get => _startTime;
            set { _startTime = value; OnPropertyChanged(); }
        }

        private DateTime _endTime;
        public DateTime EndTime
        {
            get => _endTime;
            set
            {
                _endTime = value;
                OnPropertyChanged();
                OnPropertyChanged(nameof(Duration));
                OnPropertyChanged(nameof(IsSlow));
            }
        }

        private HttpRequest _request = new();
        public HttpRequest Request
        {
            get => _request;
            set { _request = value; OnPropertyChanged(); OnPropertyChanged(nameof(Host)); OnPropertyChanged(nameof(TotalSize)); }
        }

        private HttpResponse _response = new();
        public HttpResponse Response
        {
            get => _response;
            set
            {
                _response = value;
                OnPropertyChanged();
                OnPropertyChanged(nameof(HasResponse));
                OnPropertyChanged(nameof(Duration));
                OnPropertyChanged(nameof(TotalSize));
                OnPropertyChanged(nameof(IsError));
                OnPropertyChanged(nameof(IsSlow));
            }
        }

        public TimeSpan Duration => EndTime > StartTime ? EndTime - StartTime : TimeSpan.Zero;
        public bool HasResponse => Response.StatusCode != 0;

        private bool _isHttps;
        public bool IsHttps
        {
            get => _isHttps;
            set { _isHttps = value; OnPropertyChanged(); }
        }

        public string Host => Request.Headers.GetValueOrDefault("Host", Request.Url);
        public long TotalSize => Request.Body.Length + Response.Body.Length;
        public bool IsError => Response.StatusCode >= 400;
        public bool IsSlow => Duration.TotalSeconds > 3;

        private bool _isSearchMatch;
        public bool IsSearchMatch
        {
            get => _isSearchMatch;
            set { _isSearchMatch = value; OnPropertyChanged(); }
        }

        public event PropertyChangedEventHandler? PropertyChanged;

        protected void OnPropertyChanged([CallerMemberName] string? propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}
