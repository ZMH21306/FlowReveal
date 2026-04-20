using System;
using System.Collections.Generic;
using System.Text;
using FlowReveal.Models;

namespace FlowReveal.Core.Parser
{
    public class HttpParser
    {
        public HttpMessage ParseHttpRequest(byte[] data, int offset, int length)
        {
            try
            {
                if (data == null || data.Length == 0)
                    return null;

                if (offset < 0 || length <= 0 || offset + length > data.Length)
                    return null;

                string httpData = Encoding.UTF8.GetString(data, offset, length);
                
                // 检查是否看起来像 HTTP 请求
                if (!httpData.StartsWith("GET") && !httpData.StartsWith("POST") && 
                    !httpData.StartsWith("PUT") && !httpData.StartsWith("DELETE") &&
                    !httpData.StartsWith("HEAD") && !httpData.StartsWith("OPTIONS") &&
                    !httpData.StartsWith("PATCH"))
                {
                    return null;
                }

                var lines = httpData.Split(new[] { "\r\n" }, StringSplitOptions.None);
                
                if (lines.Length == 0)
                    return null;

                var message = new HttpMessage { IsRequest = true };
                
                // 解析请求行
                var requestLine = lines[0].Split(' ');
                if (requestLine.Length >= 3)
                {
                    message.Method = requestLine[0];
                    message.Url = requestLine[1];
                    message.HttpVersion = requestLine[2];
                }

                // 解析头部
                int bodyStartIndex = 1;
                for (; bodyStartIndex < lines.Length; bodyStartIndex++)
                {
                    var line = lines[bodyStartIndex];
                    if (string.IsNullOrEmpty(line))
                        break;

                    var headerParts = line.Split(new[] { ": " }, 2, StringSplitOptions.None);
                    if (headerParts.Length == 2)
                    {
                        var key = headerParts[0];
                        var value = headerParts[1];
                        message.Headers[key] = value;

                        // 解析常见头部
                        switch (key.ToLower())
                        {
                            case "host":
                                message.Host = value;
                                break;
                            case "user-agent":
                                message.UserAgent = value;
                                break;
                            case "referer":
                                message.Referer = value;
                                break;
                            case "content-type":
                                message.ContentType = value;
                                break;
                            case "cookie":
                                ParseCookies(value, message.Cookies);
                                break;
                        }
                    }
                }

                // 解析请求体
                if (bodyStartIndex + 1 < lines.Length)
                {
                    var bodyBuilder = new StringBuilder();
                    for (int i = bodyStartIndex + 1; i < lines.Length; i++)
                    {
                        bodyBuilder.AppendLine(lines[i]);
                    }
                    message.Body = bodyBuilder.ToString();
                    message.BodySize = message.Body.Length;
                }

                // 解析查询参数
                if (!string.IsNullOrEmpty(message.Url) && message.Url.Contains('?'))
                {
                    var parts = message.Url.Split('?');
                    if (parts.Length == 2)
                    {
                        ParseQueryParameters(parts[1], message.QueryParameters);
                    }
                }

                return message;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"解析 HTTP 请求失败: {ex.Message}");
                return null;
            }
        }

        public HttpMessage ParseHttpResponse(byte[] data, int offset, int length)
        {
            try
            {
                if (data == null || data.Length == 0)
                    return null;

                if (offset < 0 || length <= 0 || offset + length > data.Length)
                    return null;

                string httpData = Encoding.UTF8.GetString(data, offset, length);
                
                // 检查是否看起来像 HTTP 响应
                if (!httpData.StartsWith("HTTP/"))
                    return null;

                var lines = httpData.Split(new[] { "\r\n" }, StringSplitOptions.None);
                
                if (lines.Length == 0)
                    return null;

                var message = new HttpMessage { IsRequest = false };
                
                // 解析状态行
                var statusLine = lines[0].Split(' ');
                if (statusLine.Length >= 3)
                {
                    message.HttpVersion = statusLine[0];
                    if (int.TryParse(statusLine[1], out int statusCode))
                    {
                        message.StatusCode = statusCode;
                    }
                    message.StatusMessage = string.Join(" ", statusLine, 2, statusLine.Length - 2);
                }

                // 解析头部
                int bodyStartIndex = 1;
                for (; bodyStartIndex < lines.Length; bodyStartIndex++)
                {
                    var line = lines[bodyStartIndex];
                    if (string.IsNullOrEmpty(line))
                        break;

                    var headerParts = line.Split(new[] { ": " }, 2, StringSplitOptions.None);
                    if (headerParts.Length == 2)
                    {
                        var key = headerParts[0];
                        var value = headerParts[1];
                        message.Headers[key] = value;

                        // 解析常见头部
                        switch (key.ToLower())
                        {
                            case "content-type":
                                message.ContentType = value;
                                break;
                            case "content-length":
                                if (long.TryParse(value, out long contentLength))
                                {
                                    message.BodySize = contentLength;
                                }
                                break;
                        }
                    }
                }

                // 解析响应体
                if (bodyStartIndex + 1 < lines.Length)
                {
                    var bodyBuilder = new StringBuilder();
                    for (int i = bodyStartIndex + 1; i < lines.Length; i++)
                    {
                        bodyBuilder.AppendLine(lines[i]);
                    }
                    message.Body = bodyBuilder.ToString();
                }

                return message;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"解析 HTTP 响应失败: {ex.Message}");
                return null;
            }
        }

        private void ParseCookies(string cookieHeader, Dictionary<string, string> cookies)
        {
            if (string.IsNullOrEmpty(cookieHeader))
                return;

            var cookiePairs = cookieHeader.Split(';');
            foreach (var pair in cookiePairs)
            {
                var cookieParts = pair.Trim().Split('=');
                if (cookieParts.Length == 2)
                {
                    cookies[cookieParts[0]] = cookieParts[1];
                }
            }
        }

        private void ParseQueryParameters(string queryString, Dictionary<string, string> parameters)
        {
            if (string.IsNullOrEmpty(queryString))
                return;

            var paramPairs = queryString.Split('&');
            foreach (var pair in paramPairs)
            {
                var paramParts = pair.Split('=');
                if (paramParts.Length == 2)
                {
                    parameters[paramParts[0]] = Uri.UnescapeDataString(paramParts[1]);
                }
            }
        }
    }
}