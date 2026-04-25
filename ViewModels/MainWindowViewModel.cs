using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text;
using System.Windows.Input;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using FlowReveal.Platforms.Windows.Network;
using Microsoft.Extensions.Logging;
using Avalonia;
using Avalonia.Threading;

namespace FlowReveal.ViewModels
{
    public partial class MainWindowViewModel : ViewModelBase
    {
        private readonly IPacketCaptureService _captureService;
        private readonly IProtocolParser _protocolParser;
        private readonly IFilterEngine _filterEngine;
        private readonly NetworkAdapterManager _adapterManager;
        private readonly ILogger<MainWindowViewModel> _logger;

        public ObservableCollection<HttpConversation> Conversations { get; } = new();
        public ObservableCollection<HttpConversation> FilteredConversations { get; } = new();

        private bool _isCapturing;
        public bool IsCapturing
        {
            get => _isCapturing;
            set => SetProperty(ref _isCapturing, value);
        }

        private string _statusText = "Ready";
        public string StatusText
        {
            get => _statusText;
            set => SetProperty(ref _statusText, value);
        }

        private string _statisticsText = "Packets: 0 | Bytes: 0";
        public string StatisticsText
        {
            get => _statisticsText;
            set => SetProperty(ref _statisticsText, value);
        }

        private string _conversationCountText = "Conversations: 0";
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

        public MainWindowViewModel(
            IPacketCaptureService captureService,
            IProtocolParser protocolParser,
            IFilterEngine filterEngine,
            NetworkAdapterManager adapterManager,
            ILogger<MainWindowViewModel> logger)
        {
            _captureService = captureService;
            _protocolParser = protocolParser;
            _filterEngine = filterEngine;
            _adapterManager = adapterManager;
            _logger = logger;

            StartCaptureCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteStartCapture);
            StopCaptureCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteStopCapture);
            ClearCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClear);
            ApplyFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteApplyFilter);
            ClearFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClearFilter);

            _captureService.StatusChanged += OnCaptureStatusChanged;
            _captureService.StatisticsUpdated += OnStatisticsUpdated;
            _captureService.PacketCaptured += OnPacketCaptured;
            _protocolParser.ConversationCreated += OnConversationCreated;
            _protocolParser.ConversationUpdated += OnConversationUpdated;

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
                    StatusText = "No network adapter available";
                    _logger.LogWarning("No network adapter available for capture");
                    return;
                }

                _logger.LogInformation("Starting capture on adapter: {AdapterName}", adapter.FriendlyName);
                await _captureService.StartCaptureAsync(adapter);
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
                _logger.LogInformation("Capture stopped by user");
            }
            catch (Exception ex)
            {
                StatusText = $"Error: {ex.Message}";
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
            ConversationCountText = "Conversations: 0";
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
                StatisticsText = $"Packets: {stats.TotalPacketsCaptured} | Bytes: {stats.TotalBytesCaptured:N0} | Rate: {stats.PacketsPerSecond:F1} pkt/s";
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
                : "(empty)";

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
                ResponseHeadersText = "(waiting for response...)";
                ResponseBodyText = "";
            }

            var timing = new StringBuilder();
            timing.AppendLine($"Start: {conv.StartTime:HH:mm:ss.fff}");
            timing.AppendLine($"End: {(conv.HasResponse ? conv.EndTime.ToString("HH:mm:ss.fff") : "pending")}");
            timing.AppendLine($"Duration: {conv.Duration.TotalMilliseconds:F1} ms");
            timing.AppendLine($"Request Size: {conv.Request.Body.Length:N0} bytes");
            timing.AppendLine($"Response Size: {conv.Response.Body.Length:N0} bytes");
            timing.AppendLine($"Total Size: {conv.TotalSize:N0} bytes");
            timing.AppendLine($"HTTPS: {(conv.IsHttps ? "Yes" : "No")}");
            TimingText = timing.ToString();
        }

        private string FormatBody(byte[] body, string contentType)
        {
            if (body.Length == 0) return "(empty)";

            try
            {
                if (contentType.Contains("json", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("xml", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("text", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("html", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("javascript", StringComparison.OrdinalIgnoreCase) ||
                    contentType.Contains("css", StringComparison.OrdinalIgnoreCase))
                {
                    return Encoding.UTF8.GetString(body);
                }

                if (body.Length > 1024)
                {
                    return $"(Binary data, {body.Length:N0} bytes - use Hex viewer for details)";
                }

                return Encoding.UTF8.GetString(body);
            }
            catch
            {
                return $"(Binary data, {body.Length:N0} bytes)";
            }
        }
    }
}
