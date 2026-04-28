using FlowReveal.Models;
using System.IO;
using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public interface IHttpParser
    {
        Task<HttpLogEntry> ParseHttpRequestAsync(Stream stream);
        Task<HttpLogEntry> ParseHttpResponseAsync(Stream stream, HttpLogEntry requestEntry);
    }
}
