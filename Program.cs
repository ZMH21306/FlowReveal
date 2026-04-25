using Avalonia;
using FlowReveal.Logging;
using Serilog;
using System;
using System.Threading.Tasks;

namespace FlowReveal
{
    internal sealed class Program
    {
        [STAThread]
        public static void Main(string[] args)
        {
            Console.OutputEncoding = System.Text.Encoding.UTF8;

            Log.Logger = LogManager.Logger;

            AppDomain.CurrentDomain.UnhandledException += (s, e) =>
            {
                Log.Fatal(e.ExceptionObject as Exception, "Unhandled exception");
            };

            TaskScheduler.UnobservedTaskException += (s, e) =>
            {
                Log.Error(e.Exception, "Unobserved task exception");
                e.SetObserved();
            };

            Log.Information("FlowReveal starting...");
            Log.Information("Version: {Version}", typeof(Program).Assembly.GetName().Version);
            Log.Information("OS: {OS}", Environment.OSVersion);
            Log.Information("Runtime: {Runtime}", Environment.Version);
            Log.Information("Machine: {Machine}", Environment.MachineName);
            Log.Information("Log directory: {LogDir}", LogManager.LogDirectory);
            Log.Information("Admin privileges: {IsAdmin}", IsRunningAsAdmin());

            try
            {
                BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
            }
            catch (Exception ex)
            {
                Log.Fatal(ex, "Application terminated unexpectedly");
                throw;
            }
            finally
            {
                Log.Information("FlowReveal shutting down");
                LogManager.CloseAndFlush();
            }
        }

        public static AppBuilder BuildAvaloniaApp()
            => AppBuilder.Configure<App>()
                .UsePlatformDetect()
                .WithInterFont()
                .LogToTrace();

        private static bool IsRunningAsAdmin()
        {
            try
            {
                var identity = System.Security.Principal.WindowsIdentity.GetCurrent();
                var principal = new System.Security.Principal.WindowsPrincipal(identity);
                return principal.IsInRole(System.Security.Principal.WindowsBuiltInRole.Administrator);
            }
            catch
            {
                return false;
            }
        }
    }
}
