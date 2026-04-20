using System.Collections.Concurrent;
using FlowReveal.Models;

namespace FlowReveal.Core.Capture
{
    public class PacketBuffer
    {
        private ConcurrentQueue<PacketInfo> _packetQueue;
        private int _maxSize;

        public int Count => _packetQueue.Count;
        public int MaxSize => _maxSize;

        public PacketBuffer(int maxSize = 10000)
        {
            _packetQueue = new ConcurrentQueue<PacketInfo>();
            _maxSize = maxSize;
        }

        public void AddPacket(PacketInfo packet)
        {
            _packetQueue.Enqueue(packet);

            // 保持队列大小在限制范围内
            while (_packetQueue.Count > _maxSize)
            {
                _packetQueue.TryDequeue(out _);
            }
        }

        public bool TryGetPacket(out PacketInfo packet)
        {
            return _packetQueue.TryDequeue(out packet);
        }

        public void Clear()
        {
            while (_packetQueue.TryDequeue(out _))
            {
                // 清空队列
            }
        }
    }
}