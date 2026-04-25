using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Text.Json;
using System.Windows.Input;
using Microsoft.Extensions.Logging;

namespace FlowReveal.ViewModels
{
    public class AppSettings
    {
        public string CaptureAdapter { get; set; } = "";
        public int CapturePort { get; set; } = 0;
        public bool EnableHttpsDecryption { get; set; } = false;
        public string Theme { get; set; } = "Default";
        public int FontSize { get; set; } = 12;
        public int SessionTimeoutMinutes { get; set; } = 5;
        public int MaxSessionBufferSize { get; set; } = 10485760;
    }

    public partial class SettingsViewModel : ViewModelBase
    {
        private readonly ILogger<SettingsViewModel> _logger;
        private readonly string _settingsPath;

        public AppSettings Settings { get; private set; } = new();

        public ObservableCollection<string> AvailableThemes { get; } = new() { "Default", "Light", "Dark" };

        public string CaptureAdapter
        {
            get => Settings.CaptureAdapter;
            set { Settings.CaptureAdapter = value; OnPropertyChanged(); }
        }

        public int CapturePort
        {
            get => Settings.CapturePort;
            set { Settings.CapturePort = value; OnPropertyChanged(); }
        }

        public bool EnableHttpsDecryption
        {
            get => Settings.EnableHttpsDecryption;
            set { Settings.EnableHttpsDecryption = value; OnPropertyChanged(); }
        }

        public string Theme
        {
            get => Settings.Theme;
            set { Settings.Theme = value; OnPropertyChanged(); }
        }

        public int FontSize
        {
            get => Settings.FontSize;
            set { Settings.FontSize = value; OnPropertyChanged(); }
        }

        public int SessionTimeoutMinutes
        {
            get => Settings.SessionTimeoutMinutes;
            set { Settings.SessionTimeoutMinutes = value; OnPropertyChanged(); }
        }

        public ICommand SaveSettingsCommand { get; }
        public ICommand LoadSettingsCommand { get; }

        public SettingsViewModel(ILogger<SettingsViewModel> logger)
        {
            _logger = logger;
            _settingsPath = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "FlowReveal", "settings.json");

            SaveSettingsCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(SaveSettings);
            LoadSettingsCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(LoadSettings);

            LoadSettings();
        }

        private void SaveSettings()
        {
            try
            {
                var dir = Path.GetDirectoryName(_settingsPath);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                    Directory.CreateDirectory(dir);

                var json = JsonSerializer.Serialize(Settings, new JsonSerializerOptions { WriteIndented = true });
                File.WriteAllText(_settingsPath, json);
                _logger.LogInformation("Settings saved to {Path}", _settingsPath);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to save settings");
            }
        }

        private void LoadSettings()
        {
            try
            {
                if (File.Exists(_settingsPath))
                {
                    var json = File.ReadAllText(_settingsPath);
                    Settings = JsonSerializer.Deserialize<AppSettings>(json) ?? new AppSettings();
                    _logger.LogInformation("Settings loaded from {Path}", _settingsPath);
                }
                else
                {
                    Settings = new AppSettings();
                    _logger.LogInformation("No settings file found, using defaults");
                }
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Failed to load settings, using defaults");
                Settings = new AppSettings();
            }

            OnPropertyChanged(nameof(CaptureAdapter));
            OnPropertyChanged(nameof(CapturePort));
            OnPropertyChanged(nameof(EnableHttpsDecryption));
            OnPropertyChanged(nameof(Theme));
            OnPropertyChanged(nameof(FontSize));
            OnPropertyChanged(nameof(SessionTimeoutMinutes));
        }
    }
}
