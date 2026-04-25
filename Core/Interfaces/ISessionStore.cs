using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using FlowReveal.Core.Models;

namespace FlowReveal.Core.Interfaces
{
    public interface ISessionStore
    {
        Task SaveSessionAsync(string filePath, IReadOnlyList<HttpConversation> conversations, CaptureStatistics statistics);
        Task<(IReadOnlyList<HttpConversation> Conversations, CaptureStatistics Statistics)> LoadSessionAsync(string filePath);
        Task ExportJsonAsync(string filePath, IReadOnlyList<HttpConversation> conversations);
        Task ExportCsvAsync(string filePath, IReadOnlyList<HttpConversation> conversations);
        Task ExportPcapAsync(string filePath, IReadOnlyList<RawPacket> packets);
    }
}
