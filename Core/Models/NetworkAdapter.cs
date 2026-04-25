using System;
using System.Collections.Generic;
using System.Net;

namespace FlowReveal.Core.Models
{
    public class NetworkAdapter
    {
        public int Index { get; set; }
        public string Name { get; set; } = string.Empty;
        public string Description { get; set; } = string.Empty;
        public string FriendlyName { get; set; } = string.Empty;
        public List<IPAddress> IpAddresses { get; set; } = new();
        public byte[] MacAddress { get; set; } = Array.Empty<byte>();
        public bool IsUp { get; set; }
        public bool IsLoopback { get; set; }
        public long Speed { get; set; }
    }
}
