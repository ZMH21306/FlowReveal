using System;
using System.Diagnostics;
using System.IO;

namespace FlowReveal.Helpers
{
    public static class DriverHelper
    {
        private const string DriverFileName = "FlowRevealDriver.sys";
        private const string DriverInfFileName = "FlowRevealDriver.inf";
        private const string DriverServiceName = "FlowRevealDriver";

        public static bool IsDriverInstalled()
        {
            try
            {
                using (var process = new Process())
                {
                    process.StartInfo.FileName = "sc";
                    process.StartInfo.Arguments = $"query {DriverServiceName}";
                    process.StartInfo.RedirectStandardOutput = true;
                    process.StartInfo.RedirectStandardError = true;
                    process.StartInfo.UseShellExecute = false;
                    process.StartInfo.CreateNoWindow = true;
                    process.Start();
                    process.WaitForExit();
                    return process.ExitCode == 0;
                }
            }
            catch
            {
                return false;
            }
        }

        public static bool InstallDriver()
        {
            try
            {
                string sysPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, DriverFileName);
                string infPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, DriverInfFileName);

                if (!File.Exists(sysPath) || !File.Exists(infPath))
                {
                    return false;
                }

                using (var process = new Process())
                {
                    process.StartInfo.FileName = "pnputil";
                    process.StartInfo.Arguments = $"/add-driver \"{infPath}\" /install";
                    process.StartInfo.Verb = "runas";
                    process.StartInfo.UseShellExecute = true;
                    process.StartInfo.CreateNoWindow = false;
                    process.Start();
                    process.WaitForExit();
                    return process.ExitCode == 0;
                }
            }
            catch
            {
                return false;
            }
        }

        public static bool UninstallDriver()
        {
            try
            {
                // 先停止服务
                using (var stopProcess = new Process())
                {
                    stopProcess.StartInfo.FileName = "sc";
                    stopProcess.StartInfo.Arguments = $"stop {DriverServiceName}";
                    stopProcess.StartInfo.UseShellExecute = false;
                    stopProcess.StartInfo.CreateNoWindow = true;
                    stopProcess.Start();
                    stopProcess.WaitForExit();
                }

                // 删除服务
                using (var deleteProcess = new Process())
                {
                    deleteProcess.StartInfo.FileName = "sc";
                    deleteProcess.StartInfo.Arguments = $"delete {DriverServiceName}";
                    deleteProcess.StartInfo.UseShellExecute = false;
                    deleteProcess.StartInfo.CreateNoWindow = true;
                    deleteProcess.Start();
                    deleteProcess.WaitForExit();
                    return deleteProcess.ExitCode == 0;
                }
            }
            catch
            {
                return false;
            }
        }

        public static bool StartDriver()
        {
            try
            {
                using (var process = new Process())
                {
                    process.StartInfo.FileName = "sc";
                    process.StartInfo.Arguments = $"start {DriverServiceName}";
                    process.StartInfo.UseShellExecute = false;
                    process.StartInfo.CreateNoWindow = true;
                    process.Start();
                    process.WaitForExit();
                    return process.ExitCode == 0;
                }
            }
            catch
            {
                return false;
            }
        }

        public static bool StopDriver()
        {
            try
            {
                using (var process = new Process())
                {
                    process.StartInfo.FileName = "sc";
                    process.StartInfo.Arguments = $"stop {DriverServiceName}";
                    process.StartInfo.UseShellExecute = false;
                    process.StartInfo.CreateNoWindow = true;
                    process.Start();
                    process.WaitForExit();
                    return process.ExitCode == 0;
                }
            }
            catch
            {
                return false;
            }
        }
    }
}
