using System.Text.Json;
using System.Text.Json.Nodes;
using Google.Protobuf;
using LichessLocal.Protocol;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.Web.WebView2.Core;
using Microsoft.UI.Windowing;
using Windows.Graphics;
using Windows.Storage.Pickers;
using Windows.UI;
using Microsoft.UI.Xaml.Media;
namespace LichessLocal;
public sealed partial class MainWindow:Window
{
 NativeCore? _core;SettingsWindow? _settings;bool _initialized,_allowClose,_closing;Snapshot? _snapshot;
 readonly JsonFormatter _formatter=new(JsonFormatter.Settings.Default.WithFormatDefaultValues(true));
 public MainWindow(){
  InitializeComponent();App.Themes.Attach(this);App.Accent.Attach(Root);AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory,"Assets","AppIcon.ico"));
  Root.Loaded+=OnLoaded;App.Themes.Changed+=ApplyTheme;App.Accent.Changed+=ApplyTheme;
  AppWindow.Closing+=(window,args)=>{if(!_allowClose){args.Cancel=true;_ = CloseSafely();}};
  AppWindow.Changed+=(_,_)=>SaveWindow();
  Closed+=(_,_)=>{App.Themes.Changed-=ApplyTheme;App.Accent.Changed-=ApplyTheme;_settings?.Close();App.Accent.Detach(Root);App.Themes.Detach(this);};
 }
 async void OnLoaded(object sender,RoutedEventArgs args){
  if(_initialized)return;_initialized=true;
  var area=DisplayArea.GetFromWindowId(AppWindow.Id,DisplayAreaFallback.Primary).WorkArea;var p=App.Preferences;
  double scale=Root.XamlRoot.RasterizationScale*new Windows.UI.ViewManagement.UISettings().TextScaleFactor;
  int w=Math.Clamp(p.Width>0?p.Width:(int)(1360*scale),Math.Min((int)(1100*scale),area.Width),area.Width),h=Math.Clamp(p.Height>0?p.Height:(int)(920*scale),Math.Min((int)(720*scale),area.Height),area.Height);
  int x=p.X==int.MinValue?area.X+(area.Width-w)/2:Math.Clamp(p.X,area.X,area.X+area.Width-w),y=p.Y==int.MinValue?area.Y+(area.Height-h)/2:Math.Clamp(p.Y,area.Y,area.Y+area.Height-h);
  AppWindow.MoveAndResize(new RectInt32(x,y,w,h));if(AppWindow.Presenter is OverlappedPresenter presenter){presenter.PreferredMinimumWidth=Math.Min((int)(1100*scale),area.Width);presenter.PreferredMinimumHeight=Math.Min((int)(720*scale),area.Height);if(p.Maximized)presenter.Maximize();}
  try{
   _core=new NativeCore(Preferences.DirectoryPath,Path.Combine(AppContext.BaseDirectory,"Engine","fairy-stockfish.exe"));
   var environment=await CoreWebView2Environment.CreateWithOptionsAsync(null,Path.Combine(Preferences.DirectoryPath,"WebView2"),null);await GameView.EnsureCoreWebView2Async(environment);
   var web=GameView.CoreWebView2;web.Settings.AreDevToolsEnabled=false;web.Settings.AreDefaultContextMenusEnabled=false;web.Settings.IsStatusBarEnabled=false;web.Settings.IsZoomControlEnabled=false;
   web.SetVirtualHostNameToFolderMapping("app.local",Path.Combine(AppContext.BaseDirectory,"Web"),CoreWebView2HostResourceAccessKind.DenyCors);
   web.AddWebResourceRequestedFilter("*",CoreWebView2WebResourceContext.All);
   web.WebResourceRequested+=(_,e)=>{if(!IsLocal(e.Request.Uri)&&!e.Request.Uri.StartsWith("data:",StringComparison.OrdinalIgnoreCase))e.Response=environment.CreateWebResourceResponse(new Windows.Storage.Streams.InMemoryRandomAccessStream(),403,"Offline application","");};
   web.NavigationStarting+=(_,e)=>{if(!IsLocal(e.Uri))e.Cancel=true;};web.PermissionRequested+=(_,e)=>e.State=CoreWebView2PermissionState.Deny;
   web.NewWindowRequested+=async(_,e)=>{e.Handled=true;if(IsLocal(e.Uri)){var file=Path.GetFileName(new Uri(e.Uri).AbsolutePath);if(file is "lila-COPYING.md" or "THIRD-PARTY.txt")await ShowCredits(file);}};
   web.WebMessageReceived+=OnWebMessage;
   web.NavigationCompleted+=(_,e)=>{if(e.IsSuccess){StartupStatus.Visibility=Visibility.Collapsed;ApplyTheme();}else{StartupStatus.Text=$"Local game view failed: {e.WebErrorStatus}";}};
   web.Navigate("https://app.local/index.html");
  }catch(Exception ex){App.Log(ex);StartupStatus.Text=$"Lichess Local could not start. {ex.Message}";}
 }
 static bool IsLocal(string uri)=>Uri.TryCreate(uri,UriKind.Absolute,out var parsed)&&parsed.Scheme=="https"&&parsed.Host=="app.local";
 async void OnWebMessage(object? sender,CoreWebView2WebMessageReceivedEventArgs args){
  if(!IsLocal(args.Source))return;string requestId="0";
  try{
   using var doc=JsonDocument.Parse(args.WebMessageAsJson);var root=doc.RootElement;if(root.TryGetProperty("requestId",out var id))requestId=id.GetString()??"0";
   string action=root.GetProperty("action").GetString()??"";
   if(action=="cancel"){_core?.Cancel();return;}
   if(action=="settings"){OpenSettings();Ack(requestId);return;}
   if(action=="pickPgn"){await PickPgn();Ack(requestId);return;}
   if(action=="savePgn"){await SavePgn(root.GetProperty("text").GetString()??"");Ack(requestId);return;}
   if(_core is null)throw new IOException("Game core is unavailable");
   if(action is "new" or "undo" or "resign" or "history" or "practice" or "import")_core.Cancel();
   var request=JsonParser.Default.Parse<Request>(args.WebMessageAsJson);var response=await _core.CallAsync(request);_snapshot=response.Snapshot;
   GameView.CoreWebView2.PostWebMessageAsJson(_formatter.Format(response));
  }catch(Exception ex){App.Log(ex);Post(new{requestId,ok=false,error=ex.Message});}
 }
 void Post(object value)=>GameView.CoreWebView2?.PostWebMessageAsJson(JsonSerializer.Serialize(value));
 void Ack(string id)=>Post(new{requestId=id,ok=true});
 void ApplyTheme(){
  if(GameView.CoreWebView2 is null)return;
  try{
   var dictionary=Application.Current.Resources.MergedDictionaries.OfType<Jupiter.UI.JupiterResources>().First();var theme=(ResourceDictionary)dictionary.ThemeDictionaries[App.Themes.IsDark?"Default":"Light"];
   string Token(string name){var c=(Color)theme[$"Jupiter{name}Color"];return $"#{c.R:X2}{c.G:X2}{c.B:X2}";}
   var a=App.Accent.Current;string accent=$"#{a.R:X2}{a.G:X2}{a.B:X2}";
   double lum=(a.R*.299+a.G*.587+a.B*.114)/255;
   Post(new{kind="theme",dark=App.Themes.IsDark,values=new Dictionary<string,string>{{"window",Token("Window")},{"surface",Token("Surface")},{"raised",Token("Raised")},{"input",Token("Input")},{"separator",Token("Separator")},{"boundary",Token("ControlBoundary")},{"text",Token("PrimaryText")},{"secondary",Token("SecondaryText")},{"hover",Token("ControlFillHover")},{"selection",Token("Selected")},{"error",Token("Error")},{"accent",accent},{"accent-text",Token(lum>.55?"Window":"PrimaryText")}}});
  }catch(Exception ex){App.Log(ex);}
 }
 async Task PickPgn(){var picker=new FileOpenPicker();WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));picker.FileTypeFilter.Add(".pgn");picker.FileTypeFilter.Add(".txt");var file=await picker.PickSingleFileAsync();if(file is null)return;var info=new FileInfo(file.Path);if(info.Length>10*1024*1024)throw new IOException("Choose a PGN file up to 10 MB.");Post(new{kind="importPgn",text=await File.ReadAllTextAsync(file.Path)});}
 async Task SavePgn(string pgn){if(string.IsNullOrWhiteSpace(pgn))return;var picker=new FileSavePicker{SuggestedFileName=$"LichessLocal-{DateTime.Now:yyyy-MM-dd}"};WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));picker.FileTypeChoices.Add("PGN game",new List<string>{".pgn"});var file=await picker.PickSaveFileAsync();if(file is not null)await File.WriteAllTextAsync(file.Path,pgn);}
 async Task ShowCredits(string file){var contents=await File.ReadAllTextAsync(Path.Combine(AppContext.BaseDirectory,"Web","notices",file));var content=new TextBlock{Text=contents,TextWrapping=TextWrapping.Wrap,IsTextSelectionEnabled=true,Style=(Style)Application.Current.Resources["JupiterBodyTextStyle"]};var dialog=new ContentDialog{XamlRoot=Root.XamlRoot,Title="Component credits and licenses",CloseButtonText="Close",Content=new ScrollViewer{Content=content,MaxHeight=520}};App.Themes.Attach(dialog);await dialog.ShowAsync();App.Themes.Detach(dialog);}
 void OpenSettings(){if(_settings is null){_settings=new SettingsWindow(this);_settings.Closed+=(_,_)=>_settings=null;}_settings.Activate();}
 void Menu(string id)=>Post(new{kind="menu",id});
 void OnNew(object sender,RoutedEventArgs e)=>Menu("new-game-menu");void OnImport(object sender,RoutedEventArgs e)=>Menu("import");void OnExport(object sender,RoutedEventArgs e)=>Menu("export");void OnSettings(object sender,RoutedEventArgs e)=>OpenSettings();void OnExit(object sender,RoutedEventArgs e)=>Close();void OnUndo(object sender,RoutedEventArgs e)=>Menu("undo");void OnClaim(object sender,RoutedEventArgs e)=>Menu("claim");void OnResign(object sender,RoutedEventArgs e)=>Menu("resign");void OnStop(object sender,RoutedEventArgs e)=>_core?.Cancel();void OnFlip(object sender,RoutedEventArgs e)=>Menu("flip");void OnHistory(object sender,RoutedEventArgs e)=>Menu("show-history");void OnAbout(object sender,RoutedEventArgs e)=>Menu("about");
 void SaveWindow(){if(!_initialized)return;var p=App.Preferences;if(AppWindow.Presenter is OverlappedPresenter presenter){p.Maximized=presenter.State==OverlappedPresenterState.Maximized;if(presenter.State==OverlappedPresenterState.Restored){p.Width=AppWindow.Size.Width;p.Height=AppWindow.Size.Height;p.X=AppWindow.Position.X;p.Y=AppWindow.Position.Y;}}p.Save();}
 async Task CloseSafely(){if(_closing)return;_closing=true;try{SaveWindow();if(_core is not null){_core.Cancel();await _core.CallAsync(new Request{Action="checkpoint"});_core.Dispose();}GameView.Close();}catch(Exception ex){App.Log(ex);}finally{_allowClose=true;Close();}}
}
