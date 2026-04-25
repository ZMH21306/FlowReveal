using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Services.Filter
{
    public class SearchResult
    {
        public HttpConversation Conversation { get; set; } = null!;
        public List<SearchMatch> Matches { get; set; } = new();
        public int TotalMatches => Matches.Count;
    }

    public class SearchMatch
    {
        public string Field { get; set; } = string.Empty;
        public int Position { get; set; }
        public string Context { get; set; } = string.Empty;
    }

    public class SearchEngine
    {
        private readonly ILogger<SearchEngine> _logger;

        public SearchEngine(ILogger<SearchEngine> logger)
        {
            _logger = logger;
        }

        public List<SearchResult> Search(IEnumerable<HttpConversation> conversations, string query, bool useRegex = false)
        {
            var results = new List<SearchResult>();

            if (string.IsNullOrWhiteSpace(query))
                return results;

            _logger.LogInformation("Searching for: {Query} (Regex: {UseRegex})", query, useRegex);

            foreach (var conv in conversations)
            {
                var matches = new List<SearchMatch>();

                if (useRegex)
                {
                    SearchInConversationRegex(conv, query, matches);
                }
                else
                {
                    SearchInConversationText(conv, query, matches);
                }

                if (matches.Count > 0)
                {
                    results.Add(new SearchResult
                    {
                        Conversation = conv,
                        Matches = matches
                    });
                }
            }

            _logger.LogInformation("Search completed: {ResultCount} conversations matched", results.Count);
            return results;
        }

        private void SearchInConversationText(HttpConversation conv, string query, List<SearchMatch> matches)
        {
            var comparison = StringComparison.OrdinalIgnoreCase;

            SearchInField(conv, "Method", conv.Request.Method, query, comparison, matches);
            SearchInField(conv, "URL", conv.Request.Url, query, comparison, matches);
            SearchInField(conv, "Host", conv.Host, query, comparison, matches);
            SearchInField(conv, "Path", conv.Request.Path, query, comparison, matches);

            foreach (var header in conv.Request.Headers)
            {
                SearchInField(conv, $"Request.Header.{header.Key}", $"{header.Key}: {header.Value}", query, comparison, matches);
            }

            foreach (var header in conv.Response.Headers)
            {
                SearchInField(conv, $"Response.Header.{header.Key}", $"{header.Key}: {header.Value}", query, comparison, matches);
            }

            if (conv.Request.Body.Length > 0)
            {
                try
                {
                    var bodyText = Encoding.UTF8.GetString(conv.Request.Body);
                    SearchInField(conv, "RequestBody", bodyText, query, comparison, matches);
                }
                catch { }
            }

            if (conv.Response.Body.Length > 0)
            {
                try
                {
                    var bodyText = Encoding.UTF8.GetString(conv.Response.Body);
                    SearchInField(conv, "ResponseBody", bodyText, query, comparison, matches);
                }
                catch { }
            }
        }

        private void SearchInField(HttpConversation conv, string field, string value, string query, StringComparison comparison, List<SearchMatch> matches)
        {
            if (string.IsNullOrEmpty(value)) return;

            int pos = 0;
            while ((pos = value.IndexOf(query, pos, comparison)) >= 0)
            {
                var contextStart = Math.Max(0, pos - 20);
                var contextEnd = Math.Min(value.Length, pos + query.Length + 20);
                var context = value[contextStart..contextEnd];

                matches.Add(new SearchMatch
                {
                    Field = field,
                    Position = pos,
                    Context = context
                });

                pos += query.Length;
            }
        }

        private void SearchInConversationRegex(HttpConversation conv, string pattern, List<SearchMatch> matches)
        {
            try
            {
                var regex = new Regex(pattern, RegexOptions.IgnoreCase, TimeSpan.FromSeconds(1));

                SearchInFieldRegex(conv, "Method", conv.Request.Method, regex, matches);
                SearchInFieldRegex(conv, "URL", conv.Request.Url, regex, matches);
                SearchInFieldRegex(conv, "Host", conv.Host, regex, matches);
                SearchInFieldRegex(conv, "Path", conv.Request.Path, regex, matches);

                foreach (var header in conv.Request.Headers)
                {
                    SearchInFieldRegex(conv, $"Request.Header.{header.Key}", $"{header.Key}: {header.Value}", regex, matches);
                }

                foreach (var header in conv.Response.Headers)
                {
                    SearchInFieldRegex(conv, $"Response.Header.{header.Key}", $"{header.Key}: {header.Value}", regex, matches);
                }

                if (conv.Request.Body.Length > 0)
                {
                    try
                    {
                        var bodyText = Encoding.UTF8.GetString(conv.Request.Body);
                        SearchInFieldRegex(conv, "RequestBody", bodyText, regex, matches);
                    }
                    catch { }
                }

                if (conv.Response.Body.Length > 0)
                {
                    try
                    {
                        var bodyText = Encoding.UTF8.GetString(conv.Response.Body);
                        SearchInFieldRegex(conv, "ResponseBody", bodyText, regex, matches);
                    }
                    catch { }
                }
            }
            catch (RegexParseException ex)
            {
                _logger.LogWarning(ex, "Invalid regex pattern: {Pattern}", pattern);
            }
        }

        private void SearchInFieldRegex(HttpConversation conv, string field, string value, Regex regex, List<SearchMatch> matches)
        {
            if (string.IsNullOrEmpty(value)) return;

            foreach (Match match in regex.Matches(value))
            {
                var contextStart = Math.Max(0, match.Index - 20);
                var contextEnd = Math.Min(value.Length, match.Index + match.Length + 20);
                var context = value[contextStart..contextEnd];

                matches.Add(new SearchMatch
                {
                    Field = field,
                    Position = match.Index,
                    Context = context
                });
            }
        }
    }
}
