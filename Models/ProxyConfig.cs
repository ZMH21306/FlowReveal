namespace FlowReveal.Models
{
    public class ProxyConfig
    {
        public int Port { get; set; } = 8888;
        public bool EnableSystemProxy { get; set; } = true;
        public bool EnableWfpRedirect { get; set; } = true;
        public int MaxBufferSize { get; set; } = 10 * 1024 * 1024; // 10MB
        public int MaxLogCount { get; set; } = 10000;
    }
}
