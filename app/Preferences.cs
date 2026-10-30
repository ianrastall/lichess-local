using System.Text.Json;
using Jupiter.UI;
namespace LichessLocal;
public sealed class Preferences:IAccentPreferences,IThemePreferences
{
 public static string DirectoryPath=>Environment.GetEnvironmentVariable("LICHESS_LOCAL_DATA")??Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),"Jupiter","LichessLocal");
 public string? Theme{get;set;} public bool UseCustomAccent{get;set;} public string? CustomAccent{get;set;}
 public int Width{get;set;}public int Height{get;set;}public int X{get;set;}=int.MinValue;public int Y{get;set;}=int.MinValue;public bool Maximized{get;set;}
 private static string FilePath=>Path.Combine(DirectoryPath,"appearance.json");
 public static Preferences Load(){try{return File.Exists(FilePath)?JsonSerializer.Deserialize<Preferences>(File.ReadAllText(FilePath))??new():new();}catch(Exception ex)when(ex is IOException or JsonException){App.Log(ex);return new();}}
 public void Save(){try{Directory.CreateDirectory(DirectoryPath);File.WriteAllText(FilePath+".tmp",JsonSerializer.Serialize(this));File.Move(FilePath+".tmp",FilePath,true);}catch(Exception ex){App.Log(ex);}}
}
