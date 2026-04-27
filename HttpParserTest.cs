using System;
using System.Text;
using FlowReveal.Services.Parser;

namespace HttpParserTest
{
    class Program
    {
        static void Main(string[] args)
        {
            Console.WriteLine("Testing HTTP Parser...");
            
            // Test 1: HTTP GET request
            var getRequest = "GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Test\r\n\r\n";
            var getBytes = Encoding.ASCII.GetBytes(getRequest);
            
            Console.WriteLine("\nTest 1: HTTP GET Request");
            Console.WriteLine("Input: " + getRequest);
            
            if (HttpParser.LooksLikeHttpRequest(getBytes, 0, getBytes.Length))
            {
                Console.WriteLine("✓ Looks like HTTP request");
                if (HttpParser.TryParseRequest(getBytes, 0, getBytes.Length, out var request, out var consumed))
                {
                    Console.WriteLine("✓ Parsed successfully");
                    Console.WriteLine($"  Method: {request.Method}");
                    Console.WriteLine($"  URL: {request.Url}");
                    Console.WriteLine($"  Host: {request.Headers.GetValueOrDefault("Host")}");
                }
                else
                {
                    Console.WriteLine("✗ Failed to parse");
                }
            }
            else
            {
                Console.WriteLine("✗ Doesn't look like HTTP request");
            }
            
            // Test 2: HTTP response
            var response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\nHello World!\n";
            var responseBytes = Encoding.ASCII.GetBytes(response);
            
            Console.WriteLine("\nTest 2: HTTP Response");
            Console.WriteLine("Input: " + response);
            
            if (HttpParser.LooksLikeHttpResponse(responseBytes, 0, responseBytes.Length))
            {
                Console.WriteLine("✓ Looks like HTTP response");
                if (HttpParser.TryParseResponse(responseBytes, 0, responseBytes.Length, out var resp, out consumed))
                {
                    Console.WriteLine("✓ Parsed successfully");
                    Console.WriteLine($"  Status: {resp.StatusCode} {resp.StatusDescription}");
                    Console.WriteLine($"  Content-Type: {resp.Headers.GetValueOrDefault("Content-Type")}");
                    Console.WriteLine($"  Body: {Encoding.ASCII.GetString(resp.Body)}");
                }
                else
                {
                    Console.WriteLine("✗ Failed to parse");
                }
            }
            else
            {
                Console.WriteLine("✗ Doesn't look like HTTP response");
            }
            
            Console.WriteLine("\nTest completed. Press any key to exit.");
            Console.ReadKey();
        }
    }
}