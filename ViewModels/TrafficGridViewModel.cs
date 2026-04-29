using Avalonia.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using FlowReveal.Models;
using FlowReveal.Services.Capture;
using FlowReveal.Services.Certificate;
using System;
using System.Collections.ObjectModel;
using System.Threading.Tasks;

namespace FlowReveal.ViewModels;

public partial class TrafficGridViewModel : ViewModelBase
{
    private readonly WfpRedirectService _wfpService;
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
        _certificateCache = new CertificateCache(new CertificateGenerator());
        _wfpService = new WfpRedirectService();
        _proxyService = new TransparentProxyService(_certificateCache);
        _proxyService.RecordCaptured += OnRecordCaptured;
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
            StopCapture();
            _proxyService?.Dispose();
            _wfpService?.Dispose();
        }

        _isDisposed = true;
    }

    [RelayCommand]
    public async Task StartCapture()
    {
        if (IsCapturing)
            return;

        EnsureCertificateInstalled();
        
        _wfpService.Start();
        await _proxyService.StartAsync();
        
        IsCapturing = true;
    }

    [RelayCommand]
    public void StopCapture()
    {
        if (!IsCapturing)
            return;

        _proxyService.Stop();
        _wfpService.Stop();
        
        IsCapturing = false;
    }

    [RelayCommand]
    public void ClearRecords()
    {
        Records.Clear();
        SelectedRecord = null;
    }

    private void OnRecordCaptured(HttpTrafficRecord record)
    {
        try
        {
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
        catch
        {
        }
    }

    private void EnsureCertificateInstalled()
    {
        var installer = new RootCAInstaller();
        
        if (!installer.IsRootCertificateInstalled())
        {
            var cert = _certificateCache.GetRootCertificate();
            installer.InstallRootCertificate(cert);
        }
    }
}