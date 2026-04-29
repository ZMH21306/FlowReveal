using Avalonia.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using FlowReveal.Models;
using FlowReveal.Services.Capture;
using FlowReveal.Services.Certificate;
using FlowReveal.Services.Logging;
using System;
using System.Collections.ObjectModel;
using System.Threading.Tasks;

namespace FlowReveal.ViewModels;

public partial class TrafficGridViewModel : ViewModelBase
{
    private readonly PortForwardingService _portForwardingService;
    private readonly TransparentProxyService _proxyService;
    private readonly CertificateCache _certificateCache;
    private bool _isDisposed;
    
    [ObservableProperty]
    private ObservableCollection<HttpTrafficRecord> _records = new();
    
    [ObservableProperty]
    private HttpTrafficRecord? _selectedRecord;
    
    [ObservableProperty]
    private bool _isCapturing;
    
    [ObservableProperty]
    private string _searchText = string.Empty;
    
    [ObservableProperty]
    private string _methodFilter = string.Empty;
    
    [ObservableProperty]
    private string _statusCodeFilter = string.Empty;

    public TrafficGridViewModel()
    {
        Logger.LogInfo("Initializing TrafficGridViewModel");
        _certificateCache = new CertificateCache(new CertificateGenerator());
        _portForwardingService = new PortForwardingService();
        _proxyService = new TransparentProxyService(_certificateCache);
        _proxyService.RecordCaptured += OnRecordCaptured;
        Logger.LogInfo("TrafficGridViewModel initialized");
    }

    ~TrafficGridViewModel()
    {
        Dispose(false);
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    protected virtual void Dispose(bool disposing)
    {
        if (_isDisposed)
            return;

        if (disposing)
        {
            Logger.LogInfo("Disposing TrafficGridViewModel");
            StopCapture();
            _proxyService?.Dispose();
            _portForwardingService?.Dispose();
        }

        _isDisposed = true;
    }

    [RelayCommand]
    public async Task StartCapture()
    {
        if (IsCapturing)
        {
            Logger.LogWarning("Capture is already running");
            return;
        }

        Logger.LogInfo("Starting capture process");
        
        try
        {
            EnsureCertificateInstalled();
            
            Logger.LogInfo("Starting port forwarding service");
            bool forwardingStarted = _portForwardingService.Start();
            
            if (!forwardingStarted)
            {
                Logger.LogError("Failed to start port forwarding service");
                return;
            }
            
            Logger.LogInfo("Starting transparent proxy service");
            await _proxyService.StartAsync();
            
            IsCapturing = true;
            Logger.LogInfo("Capture started successfully");
        }
        catch (Exception ex)
        {
            Logger.LogError("Error starting capture", ex);
        }
    }

    [RelayCommand]
    public void StopCapture()
    {
        if (!IsCapturing)
        {
            Logger.LogWarning("Capture is not running");
            return;
        }

        Logger.LogInfo("Stopping capture process");
        
        try
        {
            _proxyService.Stop();
            _portForwardingService.Stop();
            
            IsCapturing = false;
            Logger.LogInfo("Capture stopped successfully");
        }
        catch (Exception ex)
        {
            Logger.LogError("Error stopping capture", ex);
        }
    }

    [RelayCommand]
    public void ClearRecords()
    {
        Logger.LogInfo("Clearing traffic records");
        Records.Clear();
        SelectedRecord = null;
    }

    private void OnRecordCaptured(HttpTrafficRecord record)
    {
        try
        {
            Logger.LogInfo($"Record captured: {record.Method} {record.Url}");
            
            if (Dispatcher.UIThread.CheckAccess())
            {
                Records.Add(record);
                SelectedRecord = record;
            }
            else
            {
                Dispatcher.UIThread.InvokeAsync(() =>
                {
                    Records.Add(record);
                    SelectedRecord = record;
                });
            }
        }
        catch (Exception ex)
        {
            Logger.LogError("Error adding record to UI", ex);
        }
    }

    private void EnsureCertificateInstalled()
    {
        Logger.LogInfo("Checking root certificate installation");
        var installer = new RootCAInstaller();
        
        if (!installer.IsRootCertificateInstalled())
        {
            Logger.LogInfo("Root certificate not installed, installing...");
            var cert = _certificateCache.GetRootCertificate();
            bool installed = installer.InstallRootCertificate(cert);
            
            if (installed)
            {
                Logger.LogInfo("Root certificate installed successfully");
            }
            else
            {
                Logger.LogWarning("Failed to install root certificate - HTTPS decryption may not work");
            }
        }
        else
        {
            Logger.LogInfo("Root certificate already installed");
        }
    }
}