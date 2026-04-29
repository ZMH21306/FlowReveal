using CommunityToolkit.Mvvm.ComponentModel;
using FlowReveal.Models;

namespace FlowReveal.ViewModels;

public partial class DetailPanelViewModel : ViewModelBase
{
    [ObservableProperty]
    private HttpTrafficRecord? _record;

    public string RequestHeadersText => FormatHeaders(Record?.RequestHeaders);
    public string ResponseHeadersText => FormatHeaders(Record?.ResponseHeaders);
    public string RequestBodyText => Record?.RequestBodyText ?? string.Empty;
    public string ResponseBodyText => Record?.ResponseBodyText ?? string.Empty;
    public string RawRequest => Record?.RawRequest ?? string.Empty;
    public string RawResponse => Record?.RawResponse ?? string.Empty;

    private string FormatHeaders(HttpHeaders? headers)
    {
        if (headers == null)
            return string.Empty;

        var builder = new System.Text.StringBuilder();
        foreach (var header in headers)
        {
            builder.AppendLine($"{header.Name}: {header.Value}");
        }
        return builder.ToString();
    }
}