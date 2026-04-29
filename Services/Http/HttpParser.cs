using FlowReveal.Models;
using System;
using System.Buffers;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;

namespace FlowReveal.Services.Http;

public class HttpParser
{
    private readonly Encoding _defaultEncoding = Encoding.UTF8;

    public HttpTrafficRecord ParseRequestAndResponse(byte[] requestData, byte[] responseData)
    {
        var record = new HttpTrafficRecord();
        
        ParseRequest(requestData, record);
        ParseResponse(responseData, record);
        
        record.RequestSize = requestData.Length;
        record.ResponseSize = responseData.Length;
        
        record.RawRequest = _defaultEncoding.GetString(requestData);
        record.RawResponse = _defaultEncoding.GetString(responseData);
        
        return record;
    }

    public void ParseRequest(byte[] data, HttpTrafficRecord record)
    {
        if (data.Length == 0)
            return;

        string requestStr = _defaultEncoding.GetString(data);
        int headerEnd = requestStr.IndexOf("\r\n\r\n", StringComparison.Ordinal);
        
        if (headerEnd == -1)
            headerEnd = requestStr.IndexOf("\n\n", StringComparison.Ordinal);

        if (headerEnd != -1)
        {
            string headersPart = requestStr.Substring(0, headerEnd);
            string bodyPart = requestStr.Substring(headerEnd + (requestStr[headerEnd] == '\r' ? 4 : 2));

            ParseRequestHeaders(headersPart, record);
            ParseRequestBody(bodyPart, record);
        }
        else
        {
            ParseRequestHeaders(requestStr, record);
        }
    }

    private void ParseRequestHeaders(string headersStr, HttpTrafficRecord record)
    {
        string[] lines = headersStr.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
        
        if (lines.Length == 0)
            return;

        string firstLine = lines[0];
        string[] parts = firstLine.Split(new[] { ' ' }, 3);
        
        if (parts.Length >= 2)
        {
            record.Method = parts[0];
            record.Url = parts[1];
            
            if (parts.Length == 3)
                record.Protocol = parts[2];
        }

        for (int i = 1; i < lines.Length; i++)
        {
            string line = lines[i];
            int colonIndex = line.IndexOf(':');
            
            if (colonIndex != -1)
            {
                string name = line.Substring(0, colonIndex).Trim();
                string value = line.Substring(colonIndex + 1).Trim();
                record.RequestHeaders.Add(name, value);
            }
        }
    }

    private void ParseRequestBody(string bodyStr, HttpTrafficRecord record)
    {
        byte[] bodyBytes = _defaultEncoding.GetBytes(bodyStr);
        
        string contentEncoding = record.RequestHeaders["Content-Encoding"];
        record.RequestBody = DecodeBody(bodyBytes, contentEncoding);
        record.RequestBodyText = _defaultEncoding.GetString(record.RequestBody);
    }

    public void ParseResponse(byte[] data, HttpTrafficRecord record)
    {
        if (data.Length == 0)
            return;

        string responseStr = _defaultEncoding.GetString(data);
        int headerEnd = responseStr.IndexOf("\r\n\r\n", StringComparison.Ordinal);
        
        if (headerEnd == -1)
            headerEnd = responseStr.IndexOf("\n\n", StringComparison.Ordinal);

        if (headerEnd != -1)
        {
            string headersPart = responseStr.Substring(0, headerEnd);
            string bodyPart = responseStr.Substring(headerEnd + (responseStr[headerEnd] == '\r' ? 4 : 2));

            ParseResponseHeaders(headersPart, record);
            ParseResponseBody(bodyPart, record);
        }
        else
        {
            ParseResponseHeaders(responseStr, record);
        }
    }

    private void ParseResponseHeaders(string headersStr, HttpTrafficRecord record)
    {
        string[] lines = headersStr.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
        
        if (lines.Length == 0)
            return;

        string firstLine = lines[0];
        string[] parts = firstLine.Split(new[] { ' ' }, 3);
        
        if (parts.Length >= 2 && int.TryParse(parts[1], out int statusCode))
        {
            record.StatusCode = statusCode;
            
            if (parts.Length >= 3)
                record.Protocol = parts[0];
        }

        for (int i = 1; i < lines.Length; i++)
        {
            string line = lines[i];
            int colonIndex = line.IndexOf(':');
            
            if (colonIndex != -1)
            {
                string name = line.Substring(0, colonIndex).Trim();
                string value = line.Substring(colonIndex + 1).Trim();
                record.ResponseHeaders.Add(name, value);
            }
        }
    }

    private void ParseResponseBody(string bodyStr, HttpTrafficRecord record)
    {
        byte[] bodyBytes = _defaultEncoding.GetBytes(bodyStr);
        
        string transferEncoding = record.ResponseHeaders["Transfer-Encoding"];
        
        if (!string.IsNullOrEmpty(transferEncoding) && 
            transferEncoding.IndexOf("chunked", StringComparison.OrdinalIgnoreCase) >= 0)
        {
            bodyBytes = DecodeChunked(bodyBytes);
        }
        
        string contentEncoding = record.ResponseHeaders["Content-Encoding"];
        record.ResponseBody = DecodeBody(bodyBytes, contentEncoding);
        record.ResponseBodyText = _defaultEncoding.GetString(record.ResponseBody);
    }

    private byte[] DecodeChunked(byte[] data)
    {
        using var output = new MemoryStream();
        using var input = new MemoryStream(data);
        using var reader = new StreamReader(input, _defaultEncoding);
        
        string line;
        while ((line = reader.ReadLine()) != null)
        {
            line = line.Trim();
            
            if (string.IsNullOrEmpty(line))
                continue;
            
            if (!int.TryParse(line, System.Globalization.NumberStyles.HexNumber, null, out int chunkSize))
                break;
            
            if (chunkSize == 0)
                break;
            
            char[] chunk = new char[chunkSize];
            int bytesRead = reader.Read(chunk, 0, chunkSize);
            
            if (bytesRead > 0)
                output.Write(_defaultEncoding.GetBytes(chunk, 0, bytesRead));
            
            reader.ReadLine();
        }
        
        return output.ToArray();
    }

    private byte[] DecodeBody(byte[] data, string contentEncoding)
    {
        if (string.IsNullOrEmpty(contentEncoding))
            return data;

        try
        {
            contentEncoding = contentEncoding.ToLowerInvariant();
            
            if (contentEncoding.Contains("gzip"))
            {
                using var input = new MemoryStream(data);
                using var gzip = new GZipStream(input, CompressionMode.Decompress);
                using var output = new MemoryStream();
                gzip.CopyTo(output);
                return output.ToArray();
            }
            else if (contentEncoding.Contains("deflate"))
            {
                using var input = new MemoryStream(data);
                using var deflate = new DeflateStream(input, CompressionMode.Decompress);
                using var output = new MemoryStream();
                deflate.CopyTo(output);
                return output.ToArray();
            }
        }
        catch
        {
        }
        
        return data;
    }

    public byte[] ParseChunkedStream(Stream stream)
    {
        using var output = new MemoryStream();
        using var reader = new StreamReader(stream, _defaultEncoding);
        
        string line;
        while ((line = reader.ReadLine()) != null)
        {
            line = line.Trim();
            
            if (string.IsNullOrEmpty(line))
                continue;
            
            if (!int.TryParse(line, System.Globalization.NumberStyles.HexNumber, null, out int chunkSize))
                break;
            
            if (chunkSize == 0)
                break;
            
            char[] chunk = new char[chunkSize];
            int bytesRead = reader.Read(chunk, 0, chunkSize);
            
            if (bytesRead > 0)
                output.Write(_defaultEncoding.GetBytes(chunk, 0, bytesRead));
            
            reader.ReadLine();
        }
        
        return output.ToArray();
    }
}