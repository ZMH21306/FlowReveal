using FlowReveal.Helpers;
using System;
using System.Security.Principal;

namespace FlowReveal.Services
{
    public interface ILifecycleService
    {
        bool Initialize();
        void Cleanup();
        bool IsAdmin();
        bool CheckPrerequisites(out string message);
    }

    public class LifecycleService : ILifecycleService
    {
        private string _originalProxy = string.Empty;
        private bool _proxyWasSet;
        private bool _driverWasInstalled;

        public bool Initialize()
        {
            try
            {
                _originalProxy = WinProxyHelper.GetProxySettings();
                _proxyWasSet = !string.IsNullOrEmpty(_originalProxy);

                WinProxyHelper.SetProxyToLocalhost(8888);

                if (IsAdmin())
                {
                    if (!DriverHelper.IsDriverInstalled())
                    {
                        _driverWasInstalled = DriverHelper.InstallDriver();
                    }

                    if (DriverHelper.IsDriverInstalled())
                    {
                        DriverHelper.StartDriver();
                    }
                }

                return true;
            }
            catch
            {
                Cleanup();
                return false;
            }
        }

        public void Cleanup()
        {
            try
            {
                if (IsAdmin())
                {
                    DriverHelper.StopDriver();

                    if (_driverWasInstalled)
                    {
                        DriverHelper.UninstallDriver();
                    }
                }

                if (_proxyWasSet)
                {
                    WinProxyHelper.SetProxy(_originalProxy);
                }
                else
                {
                    WinProxyHelper.ClearProxy();
                }
            }
            catch
            {
            }
        }

        public bool IsAdmin()
        {
            try
            {
                using (var identity = WindowsIdentity.GetCurrent())
                {
                    var principal = new WindowsPrincipal(identity);
                    return principal.IsInRole(WindowsBuiltInRole.Administrator);
                }
            }
            catch
            {
                return false;
            }
        }

        public bool CheckPrerequisites(out string message)
        {
            message = string.Empty;

            var osVersion = Environment.OSVersion.Version;
            if (osVersion.Major < 10 || (osVersion.Major == 10 && osVersion.Build < 17763))
            {
                message = "需要 Windows 10 1809 或更高版本";
                return false;
            }

            return true;
        }
    }
}
