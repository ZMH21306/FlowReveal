using CommunityToolkit.Mvvm.ComponentModel;
using System;
using System.Collections.Generic;

namespace FlowReveal.Models;

public partial class HttpTrafficRecord : ObservableObject
{
    [ObservableProperty]
    private Guid _id = Guid.NewGuid();

    [ObservableProperty]
    private DateTime _timestamp = DateTime.Now;

    [ObservableProperty]
    private string _method = string.Empty;

    [ObservableProperty]
    private string _url = string.Empty;

    [ObservableProperty]
    private string _protocol = "HTTP/1.1";

    [ObservableProperty]
    private int _statusCode;

    [ObservableProperty]
    private long _responseTimeMs;

    [ObservableProperty]
    private long _requestSize;

    [ObservableProperty]
    private long _responseSize;

    [ObservableProperty]
    private string _remoteAddress = string.Empty;

    [ObservableProperty]
    private int _remotePort;

    [ObservableProperty]
    private string _localAddress = string.Empty;

    [ObservableProperty]
    private int _localPort;

    [ObservableProperty]
    private HttpHeaders _requestHeaders = new();

    [ObservableProperty]
    private HttpHeaders _responseHeaders = new();

    [ObservableProperty]
    private byte[] _requestBody = Array.Empty<byte>();

    [ObservableProperty]
    private byte[] _responseBody = Array.Empty<byte>();

    [ObservableProperty]
    private string _requestBodyText = string.Empty;

    [ObservableProperty]
    private string _responseBodyText = string.Empty;

    [ObservableProperty]
    private string _rawRequest = string.Empty;

    [ObservableProperty]
    private string _rawResponse = string.Empty;

    [ObservableProperty]
    private long _dnsLookupTimeMs;

    [ObservableProperty]
    private long _connectionTimeMs;

    [ObservableProperty]
    private long _tlsHandshakeTimeMs;

    [ObservableProperty]
    private long _timeToFirstByteMs;

    [ObservableProperty]
    private bool _isHttps;

    [ObservableProperty]
    private string _sni = string.Empty;

    public string RequestBodyPreview => GetPreview(_requestBodyText, 100);
    public string ResponseBodyPreview => GetPreview(_responseBodyText, 100);

    private string GetPreview(string text, int maxLength)
    {
        if (string.IsNullOrEmpty(text))
            return string.Empty;
        
        if (text.Length <= maxLength)
            return text;
        
        return text.Substring(0, maxLength) + "...";
    }
}