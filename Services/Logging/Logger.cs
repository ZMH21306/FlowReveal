using System;
using System.IO;

namespace FlowReveal.Services.Logging;

public static class Logger
{
    private static readonly string _logPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "FlowReveal",
        "debug.log"
    );

    static Logger()
    {
        var directory = Path.GetDirectoryName(_logPath);
        if (!Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory!);
        }
    }

    public static void Log(string message)
    {
        try
        {
            using var writer = new StreamWriter(_logPath, true);
            writer.WriteLine($"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] {message}");
        }
        catch
        {
        }
    }

    public static void LogError(string message, Exception? ex = null)
    {
        try
        {
            using var writer = new StreamWriter(_logPath, true);
            writer.WriteLine($"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] [ERROR] {message}");
            if (ex != null)
            {
                writer.WriteLine($"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] [ERROR] Exception: {ex}");
            }
        }
        catch
        {
        }
    }

    public static void LogInfo(string message)
    {
        Log($"[INFO] {message}");
    }

    public static void LogWarning(string message)
    {
        Log($"[WARN] {message}");
    }

    public static string GetLogPath()
    {
        return _logPath;
    }
}