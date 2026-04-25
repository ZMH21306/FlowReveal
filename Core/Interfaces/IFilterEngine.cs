using System;
using System.Collections.Generic;
using FlowReveal.Core.Models;

namespace FlowReveal.Core.Interfaces
{
    public class FilterCondition
    {
        public string Field { get; set; } = string.Empty;
        public string Operator { get; set; } = string.Empty;
        public string Value { get; set; } = string.Empty;
        public bool IsRegex { get; set; }
    }

    public class FilterGroup
    {
        public string LogicalOperator { get; set; } = "AND";
        public List<FilterCondition> Conditions { get; set; } = new();
        public List<FilterGroup> Groups { get; set; } = new();
    }

    public interface IFilterEngine
    {
        void SetFilter(FilterGroup filter);
        void ClearFilter();
        bool Matches(HttpConversation conversation);
        bool IsFilterActive { get; }
        string FilterDescription { get; }
    }
}
