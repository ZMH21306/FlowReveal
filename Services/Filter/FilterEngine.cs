using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.Services.Filter
{
    public class FilterEngine : IFilterEngine
    {
        private readonly ILogger<FilterEngine> _logger;
        private FilterGroup? _currentFilter;

        public bool IsFilterActive => _currentFilter != null;
        public string FilterDescription => _currentFilter != null ? DescribeFilter(_currentFilter) : "No filter";

        public FilterEngine(ILogger<FilterEngine> logger)
        {
            _logger = logger;
        }

        public void SetFilter(FilterGroup filter)
        {
            _currentFilter = filter;
            _logger.LogInformation("Filter set: {Description}", FilterDescription);
        }

        public void ClearFilter()
        {
            _currentFilter = null;
            _logger.LogInformation("Filter cleared");
        }

        public bool Matches(HttpConversation conversation)
        {
            if (_currentFilter == null) return true;
            return EvaluateGroup(_currentFilter, conversation);
        }

        private bool EvaluateGroup(FilterGroup group, HttpConversation conversation)
        {
            if (group.Conditions.Count == 0 && group.Groups.Count == 0)
                return true;

            var results = new List<bool>();

            foreach (var condition in group.Conditions)
            {
                results.Add(EvaluateCondition(condition, conversation));
            }

            foreach (var subGroup in group.Groups)
            {
                results.Add(EvaluateGroup(subGroup, conversation));
            }

            if (results.Count == 0)
                return true;

            if (group.LogicalOperator.Equals("OR", StringComparison.OrdinalIgnoreCase))
                return results.Any(r => r);

            return results.All(r => r);
        }

        private bool EvaluateCondition(FilterCondition condition, HttpConversation conversation)
        {
            var fieldValue = GetFieldValue(condition.Field, conversation);
            if (fieldValue == null) return false;

            var comparisonValue = condition.Value;

            if (condition.IsRegex)
            {
                try
                {
                    var regex = new Regex(comparisonValue, RegexOptions.IgnoreCase | RegexOptions.Compiled, TimeSpan.FromSeconds(1));
                    return EvaluateOperator(regex.IsMatch(fieldValue), true, condition.Operator);
                }
                catch (RegexParseException ex)
                {
                    _logger.LogWarning(ex, "Invalid regex pattern: {Pattern}", comparisonValue);
                    return false;
                }
            }

            var stringComparison = StringComparison.OrdinalIgnoreCase;
            return condition.Operator.ToUpperInvariant() switch
            {
                "EQUALS" => fieldValue.Equals(comparisonValue, stringComparison),
                "NOT_EQUALS" => !fieldValue.Equals(comparisonValue, stringComparison),
                "CONTAINS" => fieldValue.Contains(comparisonValue, stringComparison),
                "NOT_CONTAINS" => !fieldValue.Contains(comparisonValue, stringComparison),
                "STARTS_WITH" => fieldValue.StartsWith(comparisonValue, stringComparison),
                "ENDS_WITH" => fieldValue.EndsWith(comparisonValue, stringComparison),
                "GREATER_THAN" => CompareNumeric(fieldValue, comparisonValue) > 0,
                "LESS_THAN" => CompareNumeric(fieldValue, comparisonValue) < 0,
                "REGEX" => TryRegexMatch(fieldValue, comparisonValue),
                _ => fieldValue.Contains(comparisonValue, stringComparison)
            };
        }

        private bool EvaluateOperator(bool regexResult, bool expectedTrue, string op)
        {
            return op.ToUpperInvariant() switch
            {
                "NOT_EQUALS" or "NOT_CONTAINS" => !regexResult,
                _ => regexResult
            };
        }

        private string? GetFieldValue(string field, HttpConversation conversation)
        {
            return field.ToLowerInvariant() switch
            {
                "method" => conversation.Request.Method,
                "url" => conversation.Request.Url,
                "path" => conversation.Request.Path,
                "host" => conversation.Host,
                "statuscode" => conversation.Response.StatusCode.ToString(),
                "status" => conversation.Response.StatusCode.ToString(),
                "contenttype" => conversation.Response.ContentType,
                "requestcontenttype" => conversation.Request.ContentType,
                "ishttps" => conversation.IsHttps.ToString(),
                "iserror" => conversation.IsError.ToString(),
                "isslow" => conversation.IsSlow.ToString(),
                "duration" => conversation.Duration.TotalMilliseconds.ToString("F0"),
                "totalsize" => conversation.TotalSize.ToString(),
                "requestbody" => conversation.Request.Body.Length > 0 ? System.Text.Encoding.UTF8.GetString(conversation.Request.Body) : null,
                "responsebody" => conversation.Response.Body.Length > 0 ? System.Text.Encoding.UTF8.GetString(conversation.Response.Body) : null,
                _ => GetHeaderValue(field, conversation)
            };
        }

        private string? GetHeaderValue(string headerName, HttpConversation conversation)
        {
            if (conversation.Request.Headers.TryGetValue(headerName, out var reqValue))
                return reqValue;
            if (conversation.Response.Headers.TryGetValue(headerName, out var respValue))
                return respValue;
            return null;
        }

        private int CompareNumeric(string a, string b)
        {
            if (double.TryParse(a, out var numA) && double.TryParse(b, out var numB))
                return numA.CompareTo(numB);
            return string.Compare(a, b, StringComparison.OrdinalIgnoreCase);
        }

        private bool TryRegexMatch(string input, string pattern)
        {
            try
            {
                return Regex.IsMatch(input, pattern, RegexOptions.IgnoreCase, TimeSpan.FromSeconds(1));
            }
            catch
            {
                return false;
            }
        }

        private string DescribeFilter(FilterGroup group)
        {
            var parts = new List<string>();

            foreach (var condition in group.Conditions)
            {
                var regexFlag = condition.IsRegex ? " (regex)" : "";
                parts.Add($"{condition.Field} {condition.Operator} '{condition.Value}'{regexFlag}");
            }

            foreach (var subGroup in group.Groups)
            {
                parts.Add($"({DescribeFilter(subGroup)})");
            }

            var op = group.LogicalOperator.Equals("OR", StringComparison.OrdinalIgnoreCase) ? " OR " : " AND ";
            return string.Join(op, parts);
        }
    }
}
