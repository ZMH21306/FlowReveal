using System;
using System.Linq;
using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Data.Core;
using Avalonia.Data.Core.Plugins;
using Avalonia.Markup.Xaml;
using FlowReveal.Core.Interfaces;
using FlowReveal.Platforms.Windows.Capture;
using FlowReveal.Platforms.Windows.Network;
using FlowReveal.Platforms.Windows.Security;
using FlowReveal.Services.Analysis;
using FlowReveal.Services.Filter;
using FlowReveal.Services.Parser;
using FlowReveal.Services.Session;
using FlowReveal.ViewModels;
using FlowReveal.Views;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Serilog;

namespace FlowReveal
{
    public partial class App : Application
    {
        private ServiceProvider? _serviceProvider;

        public override void Initialize()
        {
            AvaloniaXamlLoader.Load(this);
        }

        public override void OnFrameworkInitializationCompleted()
        {
            if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            {
                DisableAvaloniaDataAnnotationValidation();

                var services = new ServiceCollection();
                ConfigureServices(services);
                _serviceProvider = services.BuildServiceProvider();

                var mainViewModel = new MainWindowViewModel(
                    _serviceProvider.GetRequiredService<IPacketCaptureService>(),
                    _serviceProvider.GetRequiredService<IProtocolParser>(),
                    _serviceProvider.GetRequiredService<IFilterEngine>(),
                    _serviceProvider.GetRequiredService<SearchEngine>(),
                    _serviceProvider.GetRequiredService<NetworkAdapterManager>(),
                    _serviceProvider.GetRequiredService<ILogger<MainWindowViewModel>>()
                );

                desktop.MainWindow = new MainWindow
                {
                    DataContext = mainViewModel,
                };

                desktop.Exit += OnExit;
            }

            base.OnFrameworkInitializationCompleted();
        }

        private void ConfigureServices(IServiceCollection services)
        {
            services.AddLogging(builder =>
            {
                builder.ClearProviders();
                builder.AddSerilog(Log.Logger, dispose: false);
            });

            services.AddSingleton<IProtocolParser, ProtocolParser>();
            services.AddSingleton<IFilterEngine, FilterEngine>();
            services.AddSingleton<ISessionStore, SessionStore>();
            services.AddSingleton<PrivilegeManager>();
            services.AddSingleton<CertificateManager>();
            services.AddSingleton<HttpsProxyServer>();
            services.AddSingleton<NetworkAdapterManager>();
            services.AddSingleton<IPacketCaptureService, WindowsPacketCaptureService>();
            services.AddSingleton<SearchEngine>();
            services.AddSingleton<TrafficAnalyzer>();
        }

        private void OnExit(object? sender, ControlledApplicationLifetimeExitEventArgs e)
        {
            try
            {
                var proxyServer = _serviceProvider?.GetService(typeof(HttpsProxyServer)) as HttpsProxyServer;
                if (proxyServer != null && proxyServer.IsRunning)
                {
                    proxyServer.SetSystemProxy(false);
                }
            }
            catch (Exception ex)
            {
                Log.Error(ex, "Failed to restore system proxy on exit");
            }

            Log.Information("Application exiting, disposing services");
            _serviceProvider?.Dispose();
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
