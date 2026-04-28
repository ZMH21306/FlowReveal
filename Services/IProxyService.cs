using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public interface IProxyService
    {
        Task StartAsync();
        Task StopAsync();
        bool IsRunning { get; }
    }
}
