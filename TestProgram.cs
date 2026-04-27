using System;
using System.Collections.Generic;
using System.Net;
using System.Text;
using FlowReveal.Core.Models;
using FlowReveal.Platforms.Windows.Capture;
using FlowReveal.Services.Parser;

namespace FlowReveal.Test
{
    class Program
    {
        static void Main(string[] args)
        {
            // Test HTTP parser with sample data
            Console.WriteLine("Testing HTTP parser...");
            
            // Test HTTP request parsing
            var requestData = Encoding.ASCII.GetBytes("GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Test\r\n\r\n");
            if (HttpParser.TryParseRequest(requestData, 0, requestData.Length, out var request, out var consumedBytes))
            {
                Console.WriteLine("✓ HTTP request parsed successfully");
                Console.WriteLine($"  Method: {request.Method}");
                Console.WriteLine($"  URL: {request.Url}");
                Console.WriteLine($"  Host: {request.Headers.GetValueOrDefault("Host")}");
            }
            else
            {
                Console.WriteLine("✗ HTTP request parsing failed");
            }
            
            // Test HTTP response parsing
            var responseData = Encoding.ASCII.GetBytes("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello World!\n");
            if (HttpParser.TryParseResponse(responseData, 0, responseData.Length, out var response, out consumedBytes))
            {
                Console.WriteLine("✓ HTTP response parsed successfully");
                Console.WriteLine($"  Status: {response.StatusCode} {response.StatusDescription}");
                Console.WriteLine($"  Content-Type: {response.Headers.GetValueOrDefault("Content-Type")}");
                Console.WriteLine($"  Body length: {response.Body.Length}");
            }
            else
            {
                Console.WriteLine("✗ HTTP response parsing failed");
            }
            
            Console.WriteLine("\nTest completed.");
            Console.ReadLine();
        }
    }
}