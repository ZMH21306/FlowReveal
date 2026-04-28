using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Data.Core;
using Avalonia.Data.Core.Plugins;
using Avalonia.Markup.Xaml;
using FlowReveal.Services;
using FlowReveal.ViewModels;
using FlowReveal.Views;
using System;
using System.Linq;
using System.Runtime.InteropServices;

namespace FlowReveal
{
    public partial class App : Application
    {
        private ILifecycleService _lifecycleService = new LifecycleService();

        [DllImport("user32.dll", CharSet = CharSet.Auto)]
        private static extern int MessageBox(IntPtr hWnd, string text, string caption, int options);

        private const int MB_OK = 0x00000000;
        private const int MB_ICONERROR = 0x00000010;

        public override void Initialize()
        {
            AvaloniaXamlLoader.Load(this);
        }

        public override void OnFrameworkInitializationCompleted()
        {
            if (!_lifecycleService.CheckPrerequisites(out string message))
            {
                MessageBox(IntPtr.Zero, message, "FlowReveal", MB_OK | MB_ICONERROR);
                Environment.Exit(1);
                return;
            }

            if (!_lifecycleService.Initialize())
            {
                MessageBox(IntPtr.Zero, "初始化失败", "FlowReveal", MB_OK | MB_ICONERROR);
                Environment.Exit(1);
                return;
            }

            if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            {
                DisableAvaloniaDataAnnotationValidation();
                
                desktop.MainWindow = new MainWindow
                {
                    DataContext = new MainWindowViewModel(),
                };

                desktop.Exit += OnExit;
                desktop.ShutdownRequested += OnShutdownRequested;
            }

            base.OnFrameworkInitializationCompleted();
        }

        private void OnExit(object? sender, ControlledApplicationLifetimeExitEventArgs e)
        {
            Cleanup();
        }

        private void OnShutdownRequested(object? sender, ShutdownRequestedEventArgs e)
        {
            Cleanup();
        }

        private void Cleanup()
        {
            try
            {
                _lifecycleService?.Cleanup();
            }
            catch
            {
            }
        }

        private void DisableAvaloniaDataAnnotationValidation()
        {
            var dataValidationPluginsToRemove =
                BindingPlugins.DataValidators.OfType<DataAnnotationsValidationPlugin>().ToArray();

            foreach (var plugin in dataValidationPluginsToRemove)
            {
                BindingPlugins.DataValidators.Remove(plugin);
            }
        }
    }
}