using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Xml.Linq;
using System.Windows.Input;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using FlowReveal.Platforms.Windows.Network;
using FlowReveal.Services.Filter;
using Microsoft.Extensions.Logging;
using Avalonia;
using Avalonia.Threading;

namespace FlowReveal.ViewModels
{
    public enum BodyDisplayMode
    {
        Formatted,
        Raw,
        Hex
    }

    public partial class MainWindowViewModel : ViewModelBase
    {
        private readonly IPacketCaptureService _captureService;
        private readonly IProtocolParser _protocolParser;
        private readonly IFilterEngine _filterEngine;
        private readonly SearchEngine _searchEngine;
        private readonly NetworkAdapterManager _adapterManager;
        private readonly ILogger<MainWindowViewModel> _logger;
        private readonly System.Threading.Timer _memoryTimer;
        private readonly System.Threading.Timer _cleanupTimer;
        private const int MaxConversations = 10000;

        public ObservableCollection<HttpConversation> Conversations { get; } = new();
        public ObservableCollection<HttpConversation> FilteredConversations { get; } = new();

        private bool _isCapturing;
        public bool IsCapturing
        {
            get => _isCapturing;
            set => SetProperty(ref _isCapturing, value);
        }

        private string _statusText = "就绪";
        public string StatusText
        {
            get => _statusText;
            set => SetProperty(ref _statusText, value);
        }

        private string _statisticsText = "数据包: 0 | 字节: 0";
        public string StatisticsText
        {
            get => _statisticsText;
            set => SetProperty(ref _statisticsText, value);
        }

        private string _conversationCountText = "会话: 0";
        public string ConversationCountText
        {
            get => _conversationCountText;
            set => SetProperty(ref _conversationCountText, value);
        }

        private string _memoryUsageText = "";
        public string MemoryUsageText
        {
            get => _memoryUsageText;
            set => SetProperty(ref _memoryUsageText, value);
        }

        private string _filterText = "";
        public string FilterText
        {
            get => _filterText;
            set => SetProperty(ref _filterText, value);
        }

        private string _searchText = "";
        public string SearchText
        {
            get => _searchText;
            set => SetProperty(ref _searchText, value);
        }

        private bool _isSearchActive;
        public bool IsSearchActive
        {
            get => _isSearchActive;
            set => SetProperty(ref _isSearchActive, value);
        }

        private string _searchResultCountText = "";
        public string SearchResultCountText
        {
            get => _searchResultCountText;
            set => SetProperty(ref _searchResultCountText, value);
        }

        private HttpConversation? _selectedConversation;
        public HttpConversation? SelectedConversation
        {
            get => _selectedConversation;
            set
            {
                if (SetProperty(ref _selectedConversation, value))
                {
                    UpdateDetailPanel();
                }
            }
        }

        private int _selectedDetailTab;
        public int SelectedDetailTab
        {
            get => _selectedDetailTab;
            set => SetProperty(ref _selectedDetailTab, value);
        }

        private int _selectedBodyDisplayMode = (int)BodyDisplayMode.Formatted;
        public int SelectedBodyDisplayMode
        {
            get => _selectedBodyDisplayMode;
            set
            {
                if (SetProperty(ref _selectedBodyDisplayMode, value))
                {
                    OnPropertyChanged(nameof(IsFormattedMode));
                    OnPropertyChanged(nameof(IsRawMode));
                    OnPropertyChanged(nameof(IsHexMode));
                    UpdateDetailPanel();
                }
            }
        }

        public bool IsFormattedMode
        {
            get => SelectedBodyDisplayMode == (int)BodyDisplayMode.Formatted;
            set { if (value) SelectedBodyDisplayMode = (int)BodyDisplayMode.Formatted; }
        }

        public bool IsRawMode
        {
            get => SelectedBodyDisplayMode == (int)BodyDisplayMode.Raw;
            set { if (value) SelectedBodyDisplayMode = (int)BodyDisplayMode.Raw; }
        }

        public bool IsHexMode
        {
            get => SelectedBodyDisplayMode == (int)BodyDisplayMode.Hex;
            set { if (value) SelectedBodyDisplayMode = (int)BodyDisplayMode.Hex; }
        }

        private string _requestHeadersText = "";
        public string RequestHeadersText
        {
            get => _requestHeadersText;
            set => SetProperty(ref _requestHeadersText, value);
        }

        private string _requestBodyText = "";
        public string RequestBodyText
        {
            get => _requestBodyText;
            set => SetProperty(ref _requestBodyText, value);
        }

        private string _responseHeadersText = "";
        public string ResponseHeadersText
        {
            get => _responseHeadersText;
            set => SetProperty(ref _responseHeadersText, value);
        }

        private string _responseBodyText = "";
        public string ResponseBodyText
        {
            get => _responseBodyText;
            set => SetProperty(ref _responseBodyText, value);
        }

        private string _timingText = "";
        public string TimingText
        {
            get => _timingText;
            set => SetProperty(ref _timingText, value);
        }

        public ICommand StartCaptureCommand { get; }
        public ICommand StopCaptureCommand { get; }
        public ICommand ClearCommand { get; }
        public ICommand ApplyFilterCommand { get; }
        public ICommand ClearFilterCommand { get; }
        public ICommand SearchCommand { get; }
        public ICommand ClearSearchCommand { get; }

        public MainWindowViewModel(
            IPacketCaptureService captureService,
            IProtocolParser protocolParser,
            IFilterEngine filterEngine,
            SearchEngine searchEngine,
            NetworkAdapterManager adapterManager,
            ILogger<MainWindowViewModel> logger)
        {
            _captureService = captureService;
            _protocolParser = protocolParser;
            _filterEngine = filterEngine;
            _searchEngine = searchEngine;
            _adapterManager = adapterManager;
            _logger = logger;

            StartCaptureCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteStartCapture);
            StopCaptureCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteStopCapture);
            ClearCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClear);
            ApplyFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteApplyFilter);
            ClearFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClearFilter);
            SearchCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteSearch);
            ClearSearchCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClearSearch);

            _captureService.StatusChanged += OnCaptureStatusChanged;
            _captureService.StatisticsUpdated += OnStatisticsUpdated;
            _captureService.PacketCaptured += OnPacketCaptured;
            _protocolParser.ConversationCreated += OnConversationCreated;
            _protocolParser.ConversationUpdated += OnConversationUpdated;

            _memoryTimer = new System.Threading.Timer(_ => UpdateMemoryUsage(), null, TimeSpan.Zero, TimeSpan.FromSeconds(5));
            _cleanupTimer = new System.Threading.Timer(_ => CleanupOldConversations(), null, TimeSpan.FromMinutes(2), TimeSpan.FromMinutes(2));

            _logger.LogInformation("MainWindowViewModel initialized");
        }

        private async void ExecuteStartCapture()
        {
            try
            {
                var adapters = _adapterManager.RefreshAdapters();
                var adapter = _adapterManager.GetBestAdapter();

                if (adapter == null)
                {
                    StatusText = "没有可用的网络适配器";
                    _logger.LogWarning("No network adapter available for capture");
                    return;
                }

                _logger.LogInformation("Starting capture on adapter: {AdapterName}", adapter.FriendlyName);
                await _captureService.StartCaptureAsync(adapter);
                IsCapturing = true;
                StatusText = "正在捕获...";
            }
            catch (Exception ex)
            {
                StatusText = $"Error: {ex.Message}";
                _logger.LogError(ex, "Failed to start capture");
            }
        }

        private async void ExecuteStopCapture()
        {
            try
            {
                await _captureService.StopCaptureAsync();
                IsCapturing = false;
                StatusText = "已停止";
                _logger.LogInformation("Capture stopped by user");
            }
            catch (Exception ex)
            {
                IsCapturing = false;
                StatusText = $"错误: {ex.Message}";
                _logger.LogError(ex, "Failed to stop capture");
            }
        }

        private void ExecuteClear()
        {
            Conversations.Clear();
            FilteredConversations.Clear();
            _protocolParser.Clear();
            SelectedConversation = null;
            UpdateDetailPanel();
            ConversationCountText = "会话: 0";
            _logger.LogInformation("Conversations cleared");
        }

        private void ExecuteApplyFilter()
        {
            if (string.IsNullOrWhiteSpace(FilterText))
            {
                ExecuteClearFilter();
                return;
            }

            var filter = ParseFilterText(FilterText);
            _filterEngine.SetFilter(filter);
            ApplyCurrentFilter();
            _logger.LogInformation("Filter applied: {Filter}", FilterText);
        }

        private void ExecuteClearFilter()
        {
            _filterEngine.ClearFilter();
            FilterText = "";
            ApplyCurrentFilter();
        }

        private void ExecuteSearch()
        {
            if (string.IsNullOrWhiteSpace(SearchText))
            {
                ExecuteClearSearch();
                return;
            }

            foreach (var conv in Conversations)
            {
                conv.IsSearchMatch = false;
            }

            var results = _searchEngine.Search(Conversations, SearchText);

            foreach (var result in results)
            {
                result.Conversation.IsSearchMatch = true;
            }

            IsSearchActive = true;
            SearchResultCountText = $"{results.Count} 个匹配结果";

            ApplyCurrentFilter();

            _logger.LogInformation("Search executed: {Query}, {Count} matches", SearchText, results.Count);
        }

        private void ExecuteClearSearch()
        {
            foreach (var conv in Conversations)
            {
                conv.IsSearchMatch = false;
            }

            SearchText = "";
            IsSearchActive = false;
            SearchResultCountText = "";

            ApplyCurrentFilter();

            _logger.LogInformation("Search cleared");
        }

        private FilterGroup ParseFilterText(string text)
        {
            var group = new FilterGroup { LogicalOperator = "AND" };

            var parts = text.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries);
            foreach (var part in parts)
            {
                if (part.Contains(':'))
                {
                    var colonIndex = part.IndexOf(':');
                    var field = part.Substring(0, colonIndex);
                    var value = part.Substring(colonIndex + 1);
                    group.Conditions.Add(new FilterCondition
                    {
                        Field = field,
                        Operator = "CONTAINS",
                        Value = value
                    });
                }
                else
                {
                    group.Conditions.Add(new FilterCondition
                    {
                        Field = "url",
                        Operator = "CONTAINS",
                        Value = part
                    });
                }
            }

            return group;
        }

        private void ApplyCurrentFilter()
        {
            FilteredConversations.Clear();
            foreach (var conv in Conversations)
            {
                if (_filterEngine.Matches(conv))
                {
                    FilteredConversations.Add(conv);
                }
            }
            ConversationCountText = $"Conversations: {FilteredConversations.Count}/{Conversations.Count}";
        }

        private void OnCaptureStatusChanged(object? sender, string status)
        {
            Dispatcher.UIThread.Post(() =>
            {
                StatusText = status;
            });
            _logger.LogInformation("Capture status changed: {Status}", status);
        }

        private void OnPacketCaptured(object? sender, RawPacket packet)
        {
            _protocolParser.ProcessPacket(packet);
        }

        private void OnStatisticsUpdated(object? sender, CaptureStatistics stats)
        {
            Dispatcher.UIThread.Post(() =>
            {
                StatisticsText = $"数据包: {stats.TotalPacketsCaptured} | 字节: {stats.TotalBytesCaptured:N0} | 速率: {stats.PacketsPerSecond:F1} 包/秒";
            });
        }

        private void OnConversationCreated(object? sender, HttpConversation conversation)
        {
            Dispatcher.UIThread.Post(() =>
            {
                Conversations.Add(conversation);
                if (_filterEngine.Matches(conversation))
                {
                    FilteredConversations.Add(conversation);
                }
                ConversationCountText = $"Conversations: {FilteredConversations.Count}/{Conversations.Count}";
            });
        }

        private void OnConversationUpdated(object? sender, HttpConversation conversation)
        {
            Dispatcher.UIThread.Post(() =>
            {
                if (SelectedConversation?.Id == conversation.Id)
                {
                    UpdateDetailPanel();
                }
            });
        }

        private void UpdateDetailPanel()
        {
            if (SelectedConversation == null)
            {
                RequestHeadersText = "";
                RequestBodyText = "";
                ResponseHeadersText = "";
                ResponseBodyText = "";
                TimingText = "";
                return;
            }

            var conv = SelectedConversation;

            var reqHeaders = new StringBuilder();
            reqHeaders.AppendLine($"{conv.Request.Method} {conv.Request.Url} {conv.Request.HttpVersion}");
            foreach (var header in conv.Request.Headers)
            {
                reqHeaders.AppendLine($"{header.Key}: {header.Value}");
            }
            RequestHeadersText = reqHeaders.ToString();

            RequestBodyText = conv.Request.Body.Length > 0
                ? FormatBody(conv.Request.Body, conv.Request.ContentType)
                : "(空)";

            if (conv.HasResponse)
            {
                var respHeaders = new StringBuilder();
                respHeaders.AppendLine($"{conv.Response.HttpVersion} {conv.Response.StatusCode} {conv.Response.StatusDescription}");
                foreach (var header in conv.Response.Headers)
                {
                    respHeaders.AppendLine($"{header.Key}: {header.Value}");
                }
                ResponseHeadersText = respHeaders.ToString();

                ResponseBodyText = conv.Response.Body.Length > 0
                    ? FormatBody(conv.Response.Body, conv.Response.ContentType)
                    : "(empty)";
            }
            else
            {
                ResponseHeadersText = "(等待响应...)";
                ResponseBodyText = "";
            }

            var timing = new StringBuilder();
            timing.AppendLine($"开始时间: {conv.StartTime:HH:mm:ss.fff}");
            timing.AppendLine($"结束时间: {(conv.HasResponse ? conv.EndTime.ToString("HH:mm:ss.fff") : "等待中")}");
            timing.AppendLine($"持续时间: {conv.Duration.TotalMilliseconds:F1} ms");
            timing.AppendLine($"请求大小: {conv.Request.Body.Length:N0} 字节");
            timing.AppendLine($"响应大小: {conv.Response.Body.Length:N0} 字节");
            timing.AppendLine($"总大小: {conv.TotalSize:N0} 字节");
            timing.AppendLine($"HTTPS: {(conv.IsHttps ? "是" : "否")}");
            TimingText = timing.ToString();
        }

        private void UpdateMemoryUsage()
        {
            var process = System.Diagnostics.Process.GetCurrentProcess();
            var memoryMB = process.WorkingSet64 / (1024.0 * 1024.0);
            Dispatcher.UIThread.Post(() =>
            {
                MemoryUsageText = $"内存: {memoryMB:F1} MB";
            });
        }

        private void CleanupOldConversations()
        {
            if (Conversations.Count <= MaxConversations) return;

            Dispatcher.UIThread.Post(() =>
            {
                var removeCount = Conversations.Count - MaxConversations;
                for (int i = 0; i < removeCount; i++)
                {
                    if (Conversations.Count > 0)
                    {
                        var conv = Conversations[0];
                        Conversations.RemoveAt(0);
                        FilteredConversations.Remove(conv);
                    }
                }
                ConversationCountText = $"Conversations: {FilteredConversations.Count}/{Conversations.Count}";
                _logger.LogInformation("Cleaned up {Count} old conversations, remaining: {Total}", removeCount, Conversations.Count);
            });
        }

        private string FormatBody(byte[] body, string contentType)
        {
            if (body.Length == 0) return "(空)";

            var mode = (BodyDisplayMode)SelectedBodyDisplayMode;

            if (mode == BodyDisplayMode.Hex)
            {
                return FormatHexBody(body);
            }

            try
            {
                var text = Encoding.UTF8.GetString(body);

                if (mode == BodyDisplayMode.Raw)
                {
                    return text;
                }

                if (contentType.Contains("json", StringComparison.OrdinalIgnoreCase))
                {
                    return FormatJsonBody(text);
                }

                if (contentType.Contains("xml", StringComparison.OrdinalIgnoreCase))
                {
                    return FormatXmlBody(text);
                }

                if (contentType.Contains("text", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("html", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("javascript", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("css", StringComparison.OrdinalIgnoreCase))
                {
                    return text;
                }

                return text;
            }
            catch
            {
                return FormatHexBody(body);
            }
        }

        private static string FormatJsonBody(string json)
        {
            try
            {
                var doc = JsonDocument.Parse(json);
                var options = new JsonWriterOptions { Indented = true };
                using var stream = new System.IO.MemoryStream();
                using var writer = new Utf8JsonWriter(stream, options);
                doc.WriteTo(writer);
                writer.Flush();
                return Encoding.UTF8.GetString(stream.ToArray());
            }
            catch
            {
                return json;
            }
        }

        private static string FormatXmlBody(string xml)
        {
            try
            {
                var doc = XDocument.Parse(xml);
                return doc.ToString();
            }
            catch
            {
                return xml;
            }
        }

        private static string FormatHexBody(byte[] data)
        {
            if (data.Length == 0) return "(empty)";

            var sb = new StringBuilder();
            var lines = (data.Length + 15) / 16;

            for (int i = 0; i < lines; i++)
            {
                var offset = i * 16;
                sb.Append($"{offset:X8}  ");

                for (int j = 0; j < 16; j++)
                {
                    if (j == 8) sb.Append(' ');
                    if (offset + j < data.Length)
                    {
                        sb.Append($"{data[offset + j]:X2} ");
                    }
                    else
                    {
                        sb.Append("   ");
                    }
                }

                sb.Append(' ');

                for (int j = 0; j < 16; j++)
                {
                    if (offset + j < data.Length)
                    {
                        var b = data[offset + j];
                        sb.Append(b >= 0x20 && b <= 0x7E ? (char)b : '.');
                    }
                }

                if (i < lines - 1) sb.AppendLine();
            }

            return sb.ToString();
        }
    }
}
