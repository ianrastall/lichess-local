using Microsoft.UI.Xaml;
using Microsoft.UI.Windowing;
using Windows.Graphics;
namespace LichessLocal;
public sealed partial class SettingsWindow:Window
{
 public SettingsWindow(MainWindow main){InitializeComponent();App.Themes.Attach(this);App.Accent.Attach(Root);ThemeControl.Service=App.Themes;AccentControl.Service=App.Accent;AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory,"Assets","AppIcon.ico"));Root.Loaded+=(_,_)=>{double scale=Root.XamlRoot.RasterizationScale;var area=DisplayArea.GetFromWindowId(main.AppWindow.Id,DisplayAreaFallback.Primary).WorkArea;int width=Math.Min((int)(720*scale),area.Width),height=Math.Min((int)(700*scale),area.Height);AppWindow.MoveAndResize(new RectInt32(area.X+(area.Width-width)/2,area.Y+(area.Height-height)/2,width,height));};Closed+=(_,_)=>{App.Themes.Detach(this);App.Accent.Detach(Root);};}
}
