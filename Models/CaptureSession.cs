using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace FlowReveal.Models;

public class CaptureSession
{
    public Guid Id { get; } = Guid.NewGuid();
    public DateTime StartTime { get; } = DateTime.Now;
    public DateTime? EndTime { get; private set; }
    public bool IsActive { get; private set; } = true;
    public List<HttpTrafficRecord> Records { get; } = new();
    public int MaxRecordCount { get; set; } = 10000;

    public void AddRecord(HttpTrafficRecord record)
    {
        if (!IsActive)
            throw new InvalidOperationException("Cannot add records to a stopped session");

        Records.Add(record);

        while (Records.Count > MaxRecordCount)
        {
            Records.RemoveAt(0);
        }
    }

    public void Stop()
    {
        IsActive = false;
        EndTime = DateTime.Now;
    }

    public TimeSpan Duration => (EndTime ?? DateTime.Now) - StartTime;
}