using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Windows.Input;
using FlowReveal.Core.Interfaces;
using FlowReveal.Core.Models;
using Microsoft.Extensions.Logging;

namespace FlowReveal.ViewModels
{
    public partial class FilterViewModel : ViewModelBase
    {
        private readonly IFilterEngine _filterEngine;
        private readonly ILogger<FilterViewModel> _logger;

        public ObservableCollection<FilterCondition> Conditions { get; } = new();

        private string _logicalOperator = "AND";
        public string LogicalOperator
        {
            get => _logicalOperator;
            set => SetProperty(ref _logicalOperator, value);
        }

        private string _newField = "url";
        public string NewField
        {
            get => _newField;
            set => SetProperty(ref _newField, value);
        }

        private string _newOperator = "CONTAINS";
        public string NewOperator
        {
            get => _newOperator;
            set => SetProperty(ref _newOperator, value);
        }

        private string _newValue = "";
        public string NewValue
        {
            get => _newValue;
            set => SetProperty(ref _newValue, value);
        }

        private bool _newIsRegex;
        public bool NewIsRegex
        {
            get => _newIsRegex;
            set => SetProperty(ref _newIsRegex, value);
        }

        public ObservableCollection<string> AvailableFields { get; } = new()
        {
            "method", "url", "path", "host", "statuscode", "status",
            "contenttype", "requestcontenttype", "ishttps", "iserror",
            "isslow", "duration", "totalsize", "requestbody", "responsebody"
        };

        public ObservableCollection<string> AvailableOperators { get; } = new()
        {
            "CONTAINS", "NOT_CONTAINS", "EQUALS", "NOT_EQUALS",
            "STARTS_WITH", "ENDS_WITH", "GREATER_THAN", "LESS_THAN", "REGEX"
        };

        public ICommand AddConditionCommand { get; }
        public ICommand RemoveConditionCommand { get; }
        public ICommand ApplyFilterCommand { get; }
        public ICommand ClearFilterCommand { get; }

        public event EventHandler? FilterApplied;
        public event EventHandler? FilterCleared;

        public FilterViewModel(IFilterEngine filterEngine, ILogger<FilterViewModel> logger)
        {
            _filterEngine = filterEngine;
            _logger = logger;

            AddConditionCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteAddCondition);
            RemoveConditionCommand = new CommunityToolkit.Mvvm.Input.RelayCommand<FilterCondition>(ExecuteRemoveCondition);
            ApplyFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteApplyFilter);
            ClearFilterCommand = new CommunityToolkit.Mvvm.Input.RelayCommand(ExecuteClearFilter);
        }

        private void ExecuteAddCondition()
        {
            if (string.IsNullOrWhiteSpace(NewValue)) return;

            Conditions.Add(new FilterCondition
            {
                Field = NewField,
                Operator = NewOperator,
                Value = NewValue,
                IsRegex = NewIsRegex
            });

            NewValue = "";
            _logger.LogDebug("Filter condition added: {Field} {Operator} '{Value}'", NewField, NewOperator, NewValue);
        }

        private void ExecuteRemoveCondition(FilterCondition? condition)
        {
            if (condition != null)
            {
                Conditions.Remove(condition);
                _logger.LogDebug("Filter condition removed");
            }
        }

        private void ExecuteApplyFilter()
        {
            var group = new FilterGroup
            {
                LogicalOperator = LogicalOperator,
                Conditions = Conditions.ToList()
            };

            _filterEngine.SetFilter(group);
            FilterApplied?.Invoke(this, EventArgs.Empty);
            _logger.LogInformation("Filter applied with {Count} conditions", Conditions.Count);
        }

        private void ExecuteClearFilter()
        {
            Conditions.Clear();
            _filterEngine.ClearFilter();
            FilterCleared?.Invoke(this, EventArgs.Empty);
            _logger.LogInformation("Filter cleared");
        }
    }
}
