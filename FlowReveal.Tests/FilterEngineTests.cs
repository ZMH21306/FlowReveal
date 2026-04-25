using System.Text;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using FlowReveal.Services.Filter;
using Microsoft.Extensions.Logging.Abstractions;

namespace FlowReveal.Tests;

public class FilterEngineTests
{
    private static HttpConversation CreateTestConversation(
        string method = "GET",
        string url = "https://example.com/api/test",
        int statusCode = 200,
        string? contentType = "application/json",
        string? requestBody = null,
        string? responseBody = null,
        long totalSize = 1024,
        double durationMs = 100,
        bool isHttps = true)
    {
        var request = new HttpRequest
        {
            Method = method,
            Url = url,
            Path = url.Contains('?') ? url.Substring(0, url.IndexOf('?')) : url,
            Headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                ["Host"] = new Uri(url).Host
            },
            Body = requestBody != null ? Encoding.UTF8.GetBytes(requestBody) : Array.Empty<byte>()
        };

        var response = new HttpResponse
        {
            StatusCode = statusCode,
            Headers = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
        };

        if (contentType != null)
            response.Headers["Content-Type"] = contentType;

        response.Body = responseBody != null ? Encoding.UTF8.GetBytes(responseBody) : Array.Empty<byte>();

        var conversation = new HttpConversation
        {
            Request = request,
            Response = response,
            IsHttps = isHttps,
            StartTime = DateTime.UtcNow,
            EndTime = DateTime.UtcNow.AddMilliseconds(durationMs)
        };

        return conversation;
    }

    private readonly FilterEngine _engine = new(NullLogger<FilterEngine>.Instance);

    [Fact]
    public void Matches_ContainsOperator_ReturnsTrueWhenFieldContainsValue()
    {
        var conversation = CreateTestConversation(url: "https://example.com/api/users");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "url", Operator = "CONTAINS", Value = "users" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_ContainsOperator_ReturnsFalseWhenFieldDoesNotContainValue()
    {
        var conversation = CreateTestConversation(url: "https://example.com/api/products");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "url", Operator = "CONTAINS", Value = "users" }
            }
        });

        Assert.False(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_EqualsOperator_ReturnsTrueWhenFieldEqualsValue()
    {
        var conversation = CreateTestConversation(method: "POST");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "POST" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_EqualsOperator_ReturnsFalseWhenFieldDoesNotEqualValue()
    {
        var conversation = CreateTestConversation(method: "GET");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "POST" }
            }
        });

        Assert.False(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_NotContainsOperator_ReturnsTrueWhenFieldDoesNotContainValue()
    {
        var conversation = CreateTestConversation(url: "https://example.com/api/products");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "url", Operator = "NOT_CONTAINS", Value = "users" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_GreaterThanOperator_NumericComparison_ReturnsTrue()
    {
        var conversation = CreateTestConversation(statusCode: 404);

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "statuscode", Operator = "GREATER_THAN", Value = "399" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_LessThanOperator_NumericComparison_ReturnsTrue()
    {
        var conversation = CreateTestConversation(statusCode: 200);

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "statuscode", Operator = "LESS_THAN", Value = "300" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_AndLogicalOperator_AllConditionsMustMatch()
    {
        var conversation = CreateTestConversation(method: "POST", statusCode: 201);

        _engine.SetFilter(new FilterGroup
        {
            LogicalOperator = "AND",
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "POST" },
                new() { Field = "statuscode", Operator = "EQUALS", Value = "201" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_AndLogicalOperator_FailsWhenOneConditionDoesNotMatch()
    {
        var conversation = CreateTestConversation(method: "POST", statusCode: 200);

        _engine.SetFilter(new FilterGroup
        {
            LogicalOperator = "AND",
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "POST" },
                new() { Field = "statuscode", Operator = "EQUALS", Value = "201" }
            }
        });

        Assert.False(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_OrLogicalOperator_OneConditionSuffices()
    {
        var conversation = CreateTestConversation(method: "POST", statusCode: 200);

        _engine.SetFilter(new FilterGroup
        {
            LogicalOperator = "OR",
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "POST" },
                new() { Field = "statuscode", Operator = "EQUALS", Value = "404" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void Matches_RegexMatching_ReturnsTrueWhenPatternMatches()
    {
        var conversation = CreateTestConversation(url: "https://example.com/api/v2/users");

        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "url", Operator = "REGEX", Value = @"api/v\d+/users" }
            }
        });

        Assert.True(_engine.Matches(conversation));
    }

    [Fact]
    public void ClearFilter_ResetsToMatchAll()
    {
        _engine.SetFilter(new FilterGroup
        {
            Conditions = new List<FilterCondition>
            {
                new() { Field = "method", Operator = "EQUALS", Value = "DELETE" }
            }
        });

        Assert.True(_engine.IsFilterActive);

        _engine.ClearFilter();

        Assert.False(_engine.IsFilterActive);

        var conversation = CreateTestConversation(method: "GET");
        Assert.True(_engine.Matches(conversation));
    }
}
