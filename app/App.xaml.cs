using Microsoft.UI.Xaml;
using System.Runtime.InteropServices;
namespace LichessLocal;
public partial class App : Application
{
 private Window? _window;
 private Mutex? _instance;
 internal static Preferences Preferences {get;private set;}=null!;
 internal static Jupiter.UI.ThemeService Themes {get;private set;}=null!;
 internal static Jupiter.UI.AccentService Accent {get;private set;}=null!;
 public App(){InitializeComponent();UnhandledException+=(_,e)=>Log(e.Exception);}
 protected override void OnLaunched(LaunchActivatedEventArgs args){
  _instance=new Mutex(true,"Local\\Jupiter.LichessLocal",out bool created);if(!created){Exit();return;}
  SetCurrentProcessExplicitAppUserModelID("Jupiter.LichessLocal");
  Preferences=LichessLocal.Preferences.Load();Themes=new(Preferences);Accent=new(Preferences);_window=new MainWindow();_window.AppWindow.Show();_window.Activate();
 }
 internal static void Log(Exception ex){try{Directory.CreateDirectory(LichessLocal.Preferences.DirectoryPath);File.AppendAllText(Path.Combine(LichessLocal.Preferences.DirectoryPath,"errors.log"),$"{DateTimeOffset.Now:O} {ex}\n");}catch{}}
 [LibraryImport("shell32.dll",StringMarshalling=StringMarshalling.Utf16)]private static partial int SetCurrentProcessExplicitAppUserModelID(string id);
}
