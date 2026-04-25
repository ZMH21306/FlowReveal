using System;
using System.Collections.Generic;
using FlowReveal.Core.Models;

namespace FlowReveal.Core.Interfaces
{
    public interface IProtocolParser
    {
        event EventHandler<HttpConversation>? ConversationCreated;
        event EventHandler<HttpConversation>? ConversationUpdated;
        void ProcessPacket(RawPacket packet);
        IReadOnlyList<HttpConversation> GetConversations();
        void Clear();
    }
}
