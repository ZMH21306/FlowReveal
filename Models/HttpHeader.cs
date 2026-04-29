using System.Collections.Generic;
using System.Linq;

namespace FlowReveal.Models;

public class HttpHeader
{
    public string Name { get; set; } = string.Empty;
    public string Value { get; set; } = string.Empty;

    public HttpHeader()
    {
    }

    public HttpHeader(string name, string value)
    {
        Name = name;
        Value = value;
    }
}

public class HttpHeaders : List<HttpHeader>
{
    public string? this[string name]
    {
        get => this.FirstOrDefault(h => h.Name.Equals(name, System.StringComparison.OrdinalIgnoreCase))?.Value;
        set
        {
            var existing = this.FirstOrDefault(h => h.Name.Equals(name, System.StringComparison.OrdinalIgnoreCase));
            if (existing != null)
            {
                existing.Value = value ?? string.Empty;
            }
            else if (value != null)
            {
                Add(new HttpHeader(name, value));
            }
        }
    }

    public void Add(string name, string value)
    {
        Add(new HttpHeader(name, value));
    }

    public bool Contains(string name)
    {
        return this.Any(h => h.Name.Equals(name, System.StringComparison.OrdinalIgnoreCase));
    }
}