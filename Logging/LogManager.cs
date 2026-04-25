using System;
using System.IO;
using Serilog;
using Serilog.Events;

namespace FlowReveal.Logging
{
    public static class LogManager
    {
        private static ILogger? _logger;
        private static string _logDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "FlowReveal", "Logs");

        public static ILogger Logger => _logger ??= CreateLogger();

        public static string LogDirectory => _logDirectory;

        public static ILogger CreateLogger()
        {
            if (!Directory.Exists(_logDirectory))
            {
                Directory.CreateDirectory(_logDirectory);
            }

            System.Console.OutputEncoding = System.Text.Encoding.UTF8;

            return new LoggerConfiguration()
                .MinimumLevel.Debug()
                .Enrich.WithProperty("Application", "FlowReveal")
                .Enrich.WithProperty("Version", GetVersion())
                .Enrich.WithProperty("MachineName", Environment.MachineName)
                .Enrich.WithThreadId()
                .Enrich.FromLogContext()
                .WriteTo.Console(
                    outputTemplate: "[{Timestamp:HH:mm:ss.fff}] [{Level:u3}] [{SourceContext}] [{ThreadId}] {Message:lj}{NewLine}{Exception}",
                    restrictedToMinimumLevel: LogEventLevel.Debug)
                .WriteTo.File(
                    path: Path.Combine(_logDirectory, "flowreveal-.log"),
                    outputTemplate: "[{Timestamp:yyyy-MM-dd HH:mm:ss.fff}] [{Level:u3}] [{SourceContext}] [{ThreadId}] {Message:lj}{NewLine}{Exception}",
                    rollingInterval: RollingInterval.Day,
                    retainedFileCountLimit: 7,
                    fileSizeLimitBytes: 50 * 1024 * 1024,
                    rollOnFileSizeLimit: true,
                    restrictedToMinimumLevel: LogEventLevel.Information)
                .CreateLogger();
        }

        public static void CloseAndFlush()
        {
            if (_logger is IDisposable disposable)
            {
                disposable.Dispose();
            }
            _logger = null;
        }

        private static string GetVersion()
        {
            return typeof(LogManager).Assembly.GetName().Version?.ToString() ?? "1.0.0.0";
        }
    }
}
