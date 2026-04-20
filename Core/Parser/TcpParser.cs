using System;

namespace FlowReveal.Core.Parser
{
    public class TcpHeader
    {
        public ushort SourcePort { get; set; }
        public ushort DestinationPort { get; set; }
        public uint SequenceNumber { get; set; }
        public uint AcknowledgmentNumber { get; set; }
        public byte HeaderLength { get; set; }
        public byte Flags { get; set; }
        public ushort WindowSize { get; set; }
        public ushort Checksum { get; set; }
        public ushort UrgentPointer { get; set; }

        public bool IsSyn => (Flags & 0x02) != 0;
        public bool IsAck => (Flags & 0x10) != 0;
        public bool IsFin => (Flags & 0x01) != 0;
        public bool IsRst => (Flags & 0x04) != 0;
        public bool IsPsh => (Flags & 0x08) != 0;
        public bool IsUrg => (Flags & 0x20) != 0;

        public bool Parse(byte[] data, int offset)
        {
            try
            {
                if (data.Length < offset + 20)
                    return false;

                SourcePort = BitConverter.ToUInt16(data, offset);
                DestinationPort = BitConverter.ToUInt16(data, offset + 2);
                SequenceNumber = BitConverter.ToUInt32(data, offset + 4);
                AcknowledgmentNumber = BitConverter.ToUInt32(data, offset + 8);
                
                // 头部长度（4位）
                HeaderLength = (byte)((data[offset + 12] >> 4) & 0x0F);
                
                Flags = data[offset + 13];
                WindowSize = BitConverter.ToUInt16(data, offset + 14);
                Checksum = BitConverter.ToUInt16(data, offset + 16);
                UrgentPointer = BitConverter.ToUInt16(data, offset + 18);

                return true;
            }
            catch
            {
                return false;
            }
        }
    }
}