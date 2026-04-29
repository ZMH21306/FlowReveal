using FlowReveal.Models;
using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

namespace FlowReveal.Services.Export;

public class HarExporter
{
    public string ExportToHar(List<HttpTrafficRecord> records)
    {
        var har = new
        {
            log = new
            {
                version = "1.2",
                creator = new
                {
                    name = "FlowReveal",
                    version = "1.0.0"
                },
                pages = new List<object>(),
                entries = records.ConvertAll(ConvertToHarEntry)
            }
        };

        return JsonSerializer.Serialize(har, new JsonSerializerOptions
        {
            WriteIndented = true,
            Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping
        });
    }

    public void ExportToFile(List<HttpTrafficRecord> records, string filePath)
    {
        string harJson = ExportToHar(records);
        File.WriteAllText(filePath, harJson);
    }

    private object ConvertToHarEntry(HttpTrafficRecord record)
    {
        return new
        {
            startedDateTime = record.Timestamp.ToString("yyyy-MM-ddTHH:mm:ss.fffZ"),
            time = record.ResponseTimeMs,
            request = new
            {
                method = record.Method,
                url = record.Url,
                httpVersion = record.Protocol,
                cookies = new List<object>(),
                headers = record.RequestHeaders.ConvertAll(h => new { name = h.Name, value = h.Value }),
                queryString = new List<object>(),
                postData = string.IsNullOrEmpty(record.RequestBodyText) ? null : new
                {
                    mimeType = record.RequestHeaders["Content-Type"] ?? "application/octet-stream",
                    text = record.RequestBodyText,
                    @params = new List<object>()
                },
                headersSize = record.RequestSize,
                bodySize = record.RequestBody.Length
            },
            response = new
            {
                status = record.StatusCode,
                statusText = GetStatusText(record.StatusCode),
                httpVersion = record.Protocol,
                cookies = new List<object>(),
                headers = record.ResponseHeaders.ConvertAll(h => new { name = h.Name, value = h.Value }),
                content = new
                {
                    size = record.ResponseBody.Length,
                    mimeType = record.ResponseHeaders["Content-Type"] ?? "application/octet-stream",
                    text = record.ResponseBodyText,
                    encoding = record.ResponseHeaders["Content-Encoding"]
                },
                headersSize = record.ResponseSize,
                bodySize = record.ResponseBody.Length
            },
            cache = new { },
            timings = new
            {
                dns = record.DnsLookupTimeMs,
                connect = record.ConnectionTimeMs,
                ssl = record.TlsHandshakeTimeMs,
                send = 0,
                wait = record.TimeToFirstByteMs,
                receive = 0,
                blocked = 0
            }
        };
    }

    private string GetStatusText(int statusCode)
    {
        return statusCode switch
        {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => string.Empty
        };
    }
}