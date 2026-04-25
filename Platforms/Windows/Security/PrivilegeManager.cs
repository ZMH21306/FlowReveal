using System;
using System.Diagnostics;
using System.Security.Principal;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Platforms.Windows.Security
{
    public enum PrivilegeLevel
    {
        Unknown,
        User,
        Administrator
    }

    public class PrivilegeManager
    {
        private readonly ILogger<PrivilegeManager> _logger;

        public PrivilegeLevel CurrentPrivilegeLevel { get; }
        public bool IsRunningAsAdmin => CurrentPrivilegeLevel == PrivilegeLevel.Administrator;

        public PrivilegeManager(ILogger<PrivilegeManager> logger)
        {
            _logger = logger;
            CurrentPrivilegeLevel = DetectPrivilegeLevel();
            _logger.LogInformation("Privilege level detected: {Level}", CurrentPrivilegeLevel);
        }

        private PrivilegeLevel DetectPrivilegeLevel()
        {
            try
            {
                using var identity = WindowsIdentity.GetCurrent();
                var principal = new WindowsPrincipal(identity);
                if (principal.IsInRole(WindowsBuiltInRole.Administrator))
                {
                    _logger.LogInformation("Running as Administrator (Elevated)");
                    return PrivilegeLevel.Administrator;
                }

                _logger.LogWarning("Running as standard user (Not elevated). Packet capture requires Administrator privileges.");
                return PrivilegeLevel.User;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to detect privilege level");
                return PrivilegeLevel.Unknown;
            }
        }

        public bool RequestElevation()
        {
            if (IsRunningAsAdmin)
            {
                _logger.LogInformation("Already running as Administrator, no elevation needed");
                return true;
            }

            try
            {
                _logger.LogInformation("Requesting UAC elevation by restarting process");

                var currentProcess = Process.GetCurrentProcess();
                var executablePath = currentProcess.MainModule?.FileName;

                if (string.IsNullOrEmpty(executablePath))
                {
                    _logger.LogError("Cannot determine executable path for elevation");
                    return false;
                }

                var startInfo = new ProcessStartInfo
                {
                    FileName = executablePath,
                    UseShellExecute = true,
                    Verb = "runas"
                };

                Process.Start(startInfo);
                _logger.LogInformation("Elevated process started, shutting down current instance");

                return true;
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to request UAC elevation");
                return false;
            }
        }

        public void EnsureAdminPrivilegesOrThrow()
        {
            if (!IsRunningAsAdmin)
            {
                _logger.LogCritical("Administrator privileges required for packet capture. Please restart the application as Administrator.");
                throw new UnauthorizedAccessException("Administrator privileges are required for packet capture operations.");
            }
            _logger.LogInformation("Administrator privileges verified");
        }
    }
}
