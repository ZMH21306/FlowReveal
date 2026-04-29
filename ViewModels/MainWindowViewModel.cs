using CommunityToolkit.Mvvm.ComponentModel;
using FlowReveal.Models;

namespace FlowReveal.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    public TrafficGridViewModel TrafficGrid { get; } = new();
    public DetailPanelViewModel DetailPanel { get; } = new();

    [ObservableProperty]
    private HttpTrafficRecord? _selectedRecord;

    partial void OnSelectedRecordChanged(HttpTrafficRecord? value)
    {
        DetailPanel.Record = value;
    }
}