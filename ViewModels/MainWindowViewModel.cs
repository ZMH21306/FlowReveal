using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using FlowReveal.Models;
using FlowReveal.Services;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Windows.Input;

namespace FlowReveal.ViewModels
{
    public partial class MainWindowViewModel : ViewModelBase
    {
        private readonly IProxyService _proxyService;
        private readonly IWfpCaptureService _wfpCaptureService;
        private readonly ICertificateService _certificateService;

        public ObservableCollection<HttpLogEntry> Logs { get; }
        public List<HttpLogEntry> FilteredLogs { get; private set; }

        [ObservableProperty]
        private string _searchText = string.Empty;

        [ObservableProperty]
        private string _methodFilter = string.Empty;

        [ObservableProperty]
        private int? _statusCodeFilter;

        [ObservableProperty]
        private string _processFilter = string.Empty;

        [ObservableProperty]
        private HttpLogEntry _selectedLog;

        [ObservableProperty]
        private bool _isRunning;

        private string _sortProperty = "Timestamp";
        private ListSortDirection _sortDirection = ListSortDirection.Descending;

        public ICommand StartCommand { get; }
        public ICommand StopCommand { get; }
        public ICommand SortCommand { get; }

        public MainWindowViewModel()
        {
            _certificateService = new CertificateService();
            _proxyService = new ProxyService(_certificateService);
            _wfpCaptureService = new WfpCaptureService();

            Logs = new ObservableCollection<HttpLogEntry>();
            FilteredLogs = new List<HttpLogEntry>();

            // 订阅请求捕获事件
            (_proxyService as ProxyService)?.RequestCaptured += OnRequestCaptured;

            // 初始化命令
            StartCommand = new RelayCommand(Start);
            StopCommand = new RelayCommand(Stop);
            SortCommand = new RelayCommand<string>(Sort);

            // 添加示例数据用于测试
            AddSampleData();
            ApplyFilters();
        }

        private void OnRequestCaptured(object sender, HttpLogEntry entry)
        {
            // 将捕获的请求添加到日志列表
            Logs.Add(entry);
            ApplyFilters();
        }

        private void Start()
        {
            // 启动服务
            _proxyService.StartAsync();
            _wfpCaptureService.StartAsync();
            IsRunning = true;
        }

        private void Stop()
        {
            // 停止服务
            _proxyService.StopAsync();
            _wfpCaptureService.StopAsync();
            IsRunning = false;
        }

        private void Sort(string propertyName)
        {
            if (string.IsNullOrEmpty(propertyName))
                return;

            if (_sortProperty == propertyName)
            {
                // 切换排序方向
                _sortDirection = _sortDirection == ListSortDirection.Ascending
                    ? ListSortDirection.Descending
                    : ListSortDirection.Ascending;
            }
            else
            {
                // 新的排序
                _sortProperty = propertyName;
                _sortDirection = ListSortDirection.Ascending;
            }

            ApplyFilters();
        }

        partial void OnSearchTextChanged(string value)
        {
            ApplyFilters();
        }

        partial void OnMethodFilterChanged(string value)
        {
            ApplyFilters();
        }

        partial void OnStatusCodeFilterChanged(int? value)
        {
            ApplyFilters();
        }

        partial void OnProcessFilterChanged(string value)
        {
            ApplyFilters();
        }

        private void ApplyFilters()
        {
            // 应用筛选
            var filtered = Logs.Where(log =>
            {
                if (!string.IsNullOrEmpty(SearchText) && !log.Url.Contains(SearchText, System.StringComparison.OrdinalIgnoreCase))
                    return false;
                if (!string.IsNullOrEmpty(MethodFilter) && log.Method != MethodFilter)
                    return false;
                if (StatusCodeFilter.HasValue && log.StatusCode != StatusCodeFilter.Value)
                    return false;
                if (!string.IsNullOrEmpty(ProcessFilter) && !log.ProcessName.Contains(ProcessFilter, System.StringComparison.OrdinalIgnoreCase))
                    return false;
                return true;
            }).ToList();

            // 应用排序
            filtered = SortLogs(filtered, _sortProperty, _sortDirection);

            FilteredLogs = filtered;
        }

        private List<HttpLogEntry> SortLogs(List<HttpLogEntry> logs, string propertyName, ListSortDirection direction)
        {
            switch (propertyName)
            {
                case "Timestamp":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.Timestamp).ToList()
                        : logs.OrderByDescending(l => l.Timestamp).ToList();
                case "Method":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.Method).ToList()
                        : logs.OrderByDescending(l => l.Method).ToList();
                case "Url":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.Url).ToList()
                        : logs.OrderByDescending(l => l.Url).ToList();
                case "StatusCode":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.StatusCode).ToList()
                        : logs.OrderByDescending(l => l.StatusCode).ToList();
                case "ResponseTimeMs":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.ResponseTimeMs).ToList()
                        : logs.OrderByDescending(l => l.ResponseTimeMs).ToList();
                case "RequestSize":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.RequestSize).ToList()
                        : logs.OrderByDescending(l => l.RequestSize).ToList();
                case "ResponseSize":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.ResponseSize).ToList()
                        : logs.OrderByDescending(l => l.ResponseSize).ToList();
                case "ProcessName":
                    return direction == ListSortDirection.Ascending
                        ? logs.OrderBy(l => l.ProcessName).ToList()
                        : logs.OrderByDescending(l => l.ProcessName).ToList();
                default:
                    return logs;
            }
        }

        private void AddSampleData()
        {
            // 添加一些示例数据用于测试 UI
            Logs.Add(new HttpLogEntry
            {
                Id = 1,
                Timestamp = System.DateTime.Now.AddMinutes(-5),
                Method = "GET",
                Url = "https://example.com",
                StatusCode = 200,
                ResponseTimeMs = 150,
                RequestSize = 100,
                ResponseSize = 1024,
                ContentType = "text/html",
                Host = "example.com",
                Scheme = "https",
                RequestHeaders = "User-Agent: Mozilla/5.0\r\nAccept: text/html",
                ResponseHeaders = "Content-Type: text/html\r\nContent-Length: 1024",
                RequestBody = string.Empty,
                ResponseBody = "<html><body><h1>Example</h1></body></html>",
                IsHttps = true,
                ProcessId = 1234,
                ProcessName = "chrome.exe"
            });

            Logs.Add(new HttpLogEntry
            {
                Id = 2,
                Timestamp = System.DateTime.Now.AddMinutes(-3),
                Method = "POST",
                Url = "https://api.example.com/users",
                StatusCode = 201,
                ResponseTimeMs = 250,
                RequestSize = 500,
                ResponseSize = 200,
                ContentType = "application/json",
                Host = "api.example.com",
                Scheme = "https",
                RequestHeaders = "Content-Type: application/json\r\nAuthorization: Bearer token",
                ResponseHeaders = "Content-Type: application/json\r\nLocation: /users/123",
                RequestBody = "{\"name\": \"Test\", \"email\": \"test@example.com\"}",
                ResponseBody = "{\"id\": 123, \"name\": \"Test\"}",
                IsHttps = true,
                ProcessId = 5678,
                ProcessName = "curl.exe"
            });
        }
    }
}
