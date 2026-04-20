using System;

namespace FlowReveal.Core.Parser
{
    public class UdpHeader
    {
        public ushort SourcePort { get; set; }
        public ushort DestinationPort { get; set; }
        public ushort Length { get; set; }
        public ushort Checksum { get; set; }

        public bool Parse(byte[] data, int offset)
        {
            try
            {
                if (data.Length < offset + 8)
                    return false;

                SourcePort = BitConverter.ToUInt16(data, offset);
                DestinationPort = BitConverter.ToUInt16(data, offset + 2);
                Length = BitConverter.ToUInt16(data, offset + 4);
                Checksum = BitConverter.ToUInt16(data, offset + 6);

                return true;
            }
            catch
            {
                return false;
            }
        }
    }
}