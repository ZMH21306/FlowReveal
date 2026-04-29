using FlowReveal.Services.Logging;
using System;
using System.Diagnostics;

namespace FlowReveal.Services.Capture;

public class PortForwardingService : IDisposable
{
    private readonly int _httpPort;
    private readonly int _httpsPort;
    private bool _isStarted;

    public PortForwardingService(int httpPort = 9080, int httpsPort = 9443)
    {
        _httpPort = httpPort;
        _httpsPort = httpsPort;
        Logger.LogInfo($"PortForwardingService initialized with HTTP port {httpPort}, HTTPS port {httpsPort}");
    }

    public bool Start()
    {
        if (_isStarted)
        {
            Logger.LogWarning("Port forwarding is already running");
            return true;
        }

        try
        {
            Logger.LogInfo("Setting up port forwarding rules");
            
            // 清除旧规则
            RemoveExistingRules();
            
            // 添加 HTTP 转发规则
            bool httpResult = AddPortForwardRule(80, _httpPort);
            
            // 添加 HTTPS 转发规则
            bool httpsResult = AddPortForwardRule(443, _httpsPort);

            if (httpResult && httpsResult)
            {
                _isStarted = true;
                Logger.LogInfo("Port forwarding rules added successfully");
                return true;
            }
            else
            {
                Logger.LogError("Failed to add port forwarding rules");
                RemoveExistingRules();
                return false;
            }
        }
        catch (Exception ex)
        {
            Logger.LogError("Error setting up port forwarding", ex);
            return false;
        }
    }

    public bool Stop()
    {
        if (!_isStarted)
        {
            Logger.LogWarning("Port forwarding is not running");
            return true;
        }

        try
        {
            Logger.LogInfo("Removing port forwarding rules");
            RemoveExistingRules();
            _isStarted = false;
            Logger.LogInfo("Port forwarding rules removed");
            return true;
        }
        catch (Exception ex)
        {
            Logger.LogError("Error removing port forwarding rules", ex);
            return false;
        }
    }

    private bool AddPortForwardRule(int externalPort, int localPort)
    {
        try
        {
            Logger.LogInfo($"Adding port forwarding rule: {externalPort} -> {localPort}");
            
            ProcessStartInfo psi = new()
            {
                FileName = "netsh",
                Arguments = $"interface portproxy add v4tov4 listenport={externalPort} listenaddress=0.0.0.0 connectport={localPort} connectaddress=127.0.0.1",
                Verb = "runas",
                CreateNoWindow = true,
                UseShellExecute = true,
                WindowStyle = ProcessWindowStyle.Hidden
            };

            using Process process = Process.Start(psi)!;
            process.WaitForExit();

            if (process.ExitCode == 0)
            {
                Logger.LogInfo($"Port forwarding rule {externalPort} -> {localPort} added successfully");
                return true;
            }
            else
            {
                Logger.LogError($"Failed to add port forwarding rule {externalPort} -> {localPort}. Exit code: {process.ExitCode}");
                return false;
            }
        }
        catch (Exception ex)
        {
            Logger.LogError($"Error adding port forwarding rule {externalPort} -> {localPort}", ex);
            return false;
        }
    }

    private void RemoveExistingRules()
    {
        try
        {
            Logger.LogInfo("Removing existing port forwarding rules");

            ProcessStartInfo psi80 = new()
            {
                FileName = "netsh",
                Arguments = "interface portproxy delete v4tov4 listenport=80 listenaddress=0.0.0.0",
                CreateNoWindow = true,
                UseShellExecute = false
            };

            ProcessStartInfo psi443 = new()
            {
                FileName = "netsh",
                Arguments = "interface portproxy delete v4tov4 listenport=443 listenaddress=0.0.0.0",
                CreateNoWindow = true,
                UseShellExecute = false
            };

            using Process process80 = Process.Start(psi80)!;
            process80.WaitForExit();

            using Process process443 = Process.Start(psi443)!;
            process443.WaitForExit();

            Logger.LogInfo("Existing port forwarding rules removed");
        }
        catch (Exception ex)
        {
            Logger.LogWarning($"Error removing existing port forwarding rules (may not exist): {ex.Message}");
        }
    }

    public void Dispose()
    {
        Stop();
    }

    public bool IsStarted => _isStarted;
}