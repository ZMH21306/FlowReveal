using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public interface IWfpCaptureService
    {
        Task StartAsync();
        Task StopAsync();
        bool IsRunning { get; }
    }
}
