using Google.Protobuf;
using LichessLocal;
using LichessLocal.Protocol;
string root=Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,"../../../../../../"));
if(args.Length>0)root=Path.GetFullPath(args[0]);
string directory=Path.Combine(Path.GetTempPath(),"lichess-local-interop-"+Guid.NewGuid());Directory.CreateDirectory(directory);
try{
 using var core=new NativeCore(directory,Path.Combine(root,"artifacts","engine","fairy-stockfish.exe"));
 var response=await core.CallAsync(new Request{Action="snapshot",RequestId=101});
 if(!response.Ok||response.RequestId!=101)throw new Exception("Initial request/response contract failed");
 response=await core.CallAsync(new Request{Action="new",Revision=response.Snapshot.Revision,RequestId=102,Options=new GameOptions{Variant="chess",Side="white",Level=3,Player="Interop",Rated=true}});
 if(!response.Ok)throw new Exception(response.Error);
 response=await core.CallAsync(new Request{Action="move",Text="e4",Revision=response.Snapshot.Revision});if(!response.Ok)throw new Exception(response.Error);
 response=await core.CallAsync(new Request{Action="computer",Revision=response.Snapshot.Revision});if(!response.Ok||response.Snapshot.Moves.Count!=2)throw new Exception(response.Error);
 var roundtrip=JsonParser.Default.Parse<Response>(new JsonFormatter(JsonFormatter.Settings.Default.WithFormatDefaultValues(true)).Format(response));if(roundtrip.Snapshot.Moves[0].San!="e4")throw new Exception("Web bridge projection failed");
 response=await core.CallAsync(new Request{Action="resign",Revision=response.Snapshot.Revision});if(!response.Ok||response.Snapshot.Progress.Rating.Games!=1)throw new Exception("Glicko-2 result settlement failed");
 response=await core.CallAsync(new Request{Action="analyze",Index=2,Revision=response.Snapshot.Revision});if(!response.Ok||!response.Snapshot.Analysis.Contains("bestmove"))throw new Exception("Analysis failed");
 Console.WriteLine("Managed/native protobuf, real engine, PGN, progress settlement and JSON projection passed.");
}finally{Directory.Delete(directory,true);}
