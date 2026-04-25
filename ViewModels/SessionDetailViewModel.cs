using System;
using System.Collections.Generic;
using System.Text;
using FlowReveal.Core.Models;

namespace FlowReveal.ViewModels
{
    public partial class SessionDetailViewModel : ViewModelBase
    {
        private HttpConversation? _conversation;
        public HttpConversation? Conversation
        {
            get => _conversation;
            set
            {
                if (SetProperty(ref _conversation, value))
                {
                    OnPropertyChanged(nameof(RequestHeaders));
                    OnPropertyChanged(nameof(RequestBody));
                    OnPropertyChanged(nameof(ResponseHeaders));
                    OnPropertyChanged(nameof(ResponseBody));
                    OnPropertyChanged(nameof(TimingInfo));
                    OnPropertyChanged(nameof(HasConversation));
                    OnPropertyChanged(nameof(HasResponse));
                }
            }
        }

        public bool HasConversation => Conversation != null;
        public bool HasResponse => Conversation?.HasResponse ?? false;

        public string RequestHeaders
        {
            get
            {
                if (Conversation == null) return "";
                var sb = new StringBuilder();
                sb.AppendLine($"{Conversation.Request.Method} {Conversation.Request.Url} {Conversation.Request.HttpVersion}");
                foreach (var h in Conversation.Request.Headers)
                    sb.AppendLine($"{h.Key}: {h.Value}");
                return sb.ToString();
            }
        }

        public string RequestBody
        {
            get
            {
                if (Conversation == null || Conversation.Request.Body.Length == 0) return "(empty)";
                try { return Encoding.UTF8.GetString(Conversation.Request.Body); }
                catch { return $"(Binary data, {Conversation.Request.Body.Length:N0} bytes)"; }
            }
        }

        public string ResponseHeaders
        {
            get
            {
                if (Conversation == null || !Conversation.HasResponse) return "(waiting...)";
                var sb = new StringBuilder();
                sb.AppendLine($"{Conversation.Response.HttpVersion} {Conversation.Response.StatusCode} {Conversation.Response.StatusDescription}");
                foreach (var h in Conversation.Response.Headers)
                    sb.AppendLine($"{h.Key}: {h.Value}");
                return sb.ToString();
            }
        }

        public string ResponseBody
        {
            get
            {
                if (Conversation == null || !Conversation.HasResponse) return "";
                if (Conversation.Response.Body.Length == 0) return "(empty)";
                try { return Encoding.UTF8.GetString(Conversation.Response.Body); }
                catch { return $"(Binary data, {Conversation.Response.Body.Length:N0} bytes)"; }
            }
        }

        public string TimingInfo
        {
            get
            {
                if (Conversation == null) return "";
                var sb = new StringBuilder();
                sb.AppendLine($"Start: {Conversation.StartTime:HH:mm:ss.fff}");
                sb.AppendLine($"End: {(Conversation.HasResponse ? Conversation.EndTime.ToString("HH:mm:ss.fff") : "pending")}");
                sb.AppendLine($"Duration: {Conversation.Duration.TotalMilliseconds:F1} ms");
                sb.AppendLine($"Request Size: {Conversation.Request.Body.Length:N0} bytes");
                sb.AppendLine($"Response Size: {Conversation.Response.Body.Length:N0} bytes");
                sb.AppendLine($"Total Size: {Conversation.TotalSize:N0} bytes");
                sb.AppendLine($"HTTPS: {(Conversation.IsHttps ? "Yes" : "No")}");
                return sb.ToString();
            }
        }
    }
}
