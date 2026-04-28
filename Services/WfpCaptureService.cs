using System.Threading.Tasks;

namespace FlowReveal.Services
{
    public class WfpCaptureService : IWfpCaptureService
    {
        private bool _running;

        public bool IsRunning => _running;

        public async Task StartAsync()
        {
            _running = true;
            // 这里将实现与 WFP 内核驱动的通信
            // 暂时只做占位符实现
            await Task.Delay(100);
        }

        public Task StopAsync()
        {
            _running = false;
            // 这里将实现停止 WFP 捕获的逻辑
            return Task.CompletedTask;
        }
    }
}
