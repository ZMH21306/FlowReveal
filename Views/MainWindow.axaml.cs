using Avalonia.Controls;
using Avalonia.Media;
using FlowReveal.Core.Models;

namespace FlowReveal.Views
{
    public partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();

            var dataGrid = this.FindControl<DataGrid>("TrafficDataGrid");
            if (dataGrid != null)
            {
                dataGrid.LoadingRow += OnLoadingRow;
            }
        }

        private void OnLoadingRow(object? sender, DataGridRowEventArgs e)
        {
            if (e.Row.DataContext is HttpConversation conversation)
            {
                if (conversation.IsError)
                {
                    e.Row.Foreground = new SolidColorBrush(Color.FromRgb(0xFF, 0x44, 0x44));
                }
                else if (conversation.IsSlow)
                {
                    e.Row.Foreground = new SolidColorBrush(Color.FromRgb(0xFF, 0x88, 0x00));
                }
                else
                {
                    e.Row.Foreground = null;
                }
            }
        }
    }
}
