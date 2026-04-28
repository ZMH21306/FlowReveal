using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace FlowReveal.Helpers
{
    public static class WinProxyHelper
    {
        [DllImport("wininet.dll", SetLastError = true, CharSet = CharSet.Auto)]
        private static extern bool InternetSetOption(IntPtr hInternet, int dwOption, IntPtr lpBuffer, int dwBufferLength);

        [DllImport("wininet.dll", CharSet = CharSet.Auto, SetLastError = true)]
        private static extern bool InternetQueryOption(IntPtr hInternet, int dwOption, IntPtr lpBuffer, ref int lpdwBufferLength);

        private const int INTERNET_OPTION_PROXY = 38;
        private const int INTERNET_OPTION_SETTINGS_CHANGED = 39;
        private const int INTERNET_OPTION_REFRESH = 37;

        public static string GetProxySettings()
        {
            int bufferSize = 1024;
            IntPtr buffer = Marshal.AllocHGlobal(bufferSize);
            
            try
            {
                bool success = InternetQueryOption(IntPtr.Zero, INTERNET_OPTION_PROXY, buffer, ref bufferSize);
                if (success)
                {
                    return Marshal.PtrToStringAuto(buffer);
                }
                return string.Empty;
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        public static bool SetProxy(string proxy)
        {
            IntPtr buffer = IntPtr.Zero;
            
            try
            {
                if (!string.IsNullOrEmpty(proxy))
                {
                    byte[] bytes = Encoding.Unicode.GetBytes(proxy + "\0");
                    buffer = Marshal.AllocHGlobal(bytes.Length);
                    Marshal.Copy(bytes, 0, buffer, bytes.Length);
                }

                bool success = InternetSetOption(IntPtr.Zero, INTERNET_OPTION_PROXY, buffer, 
                    string.IsNullOrEmpty(proxy) ? 0 : Encoding.Unicode.GetByteCount(proxy + "\0"));
                
                if (success)
                {
                    InternetSetOption(IntPtr.Zero, INTERNET_OPTION_SETTINGS_CHANGED, IntPtr.Zero, 0);
                    InternetSetOption(IntPtr.Zero, INTERNET_OPTION_REFRESH, IntPtr.Zero, 0);
                }
                
                return success;
            }
            finally
            {
                if (buffer != IntPtr.Zero)
                {
                    Marshal.FreeHGlobal(buffer);
                }
            }
        }

        public static bool SetProxyToLocalhost(int port = 8888)
        {
            string proxy = $"http=127.0.0.1:{port};https=127.0.0.1:{port}";
            return SetProxy(proxy);
        }

        public static bool ClearProxy()
        {
            return SetProxy(string.Empty);
        }
    }
}
