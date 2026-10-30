using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Google.Protobuf;
using LichessLocal.Protocol;
namespace LichessLocal;
internal static unsafe partial class Native
{
 const string Library="merci_core";
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial uint ll_abi_version();
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial uint ll_create(byte* bytes,nuint length,out CoreHandle core,out ResultHandle result);
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial uint ll_call(CoreHandle core,byte* bytes,nuint length,out ResultHandle result);
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial void ll_cancel(CoreHandle core);
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial uint ll_result_bytes(ResultHandle result,out byte* bytes,out nuint length);
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial void ll_result_free(nint result);
 [LibraryImport(Library)][UnmanagedCallConv(CallConvs=[typeof(CallConvCdecl)])]internal static partial void ll_destroy(nint core);
}
internal sealed class CoreHandle():SafeHandle(nint.Zero,true){public override bool IsInvalid=>handle==nint.Zero;protected override bool ReleaseHandle(){Native.ll_destroy(handle);return true;}}
internal sealed class ResultHandle():SafeHandle(nint.Zero,true){public override bool IsInvalid=>handle==nint.Zero;protected override bool ReleaseHandle(){Native.ll_result_free(handle);return true;}
 internal unsafe Response Read(){if(Native.ll_result_bytes(this,out var ptr,out var length)!=0||length>16*1024*1024)throw new IOException("Invalid core response");return Response.Parser.ParseFrom(new ReadOnlySpan<byte>(ptr,checked((int)length)));}}
public sealed class NativeCore:IDisposable
{
 readonly CoreHandle _handle;readonly SemaphoreSlim _gate=new(1,1);
 public unsafe NativeCore(string directory,string engine){if(Native.ll_abi_version()!=1)throw new IOException("Incompatible game core");var bytes=new Config{Directory=directory,Engine=engine}.ToByteArray();fixed(byte* p=bytes){var status=Native.ll_create(p,(nuint)bytes.Length,out _handle,out var error);using(error){if(status!=0)throw new IOException(error.IsInvalid?$"Game core failed ({status})":error.Read().Error);}}}
 public void Cancel()=>Native.ll_cancel(_handle);
 public async Task<Response> CallAsync(Request request){await _gate.WaitAsync();try{return await Task.Run(()=>Call(request));}finally{_gate.Release();}}
 unsafe Response Call(Request request){var bytes=request.ToByteArray();fixed(byte* p=bytes){var status=Native.ll_call(_handle,p,(nuint)bytes.Length,out var result);using(result){if(status!=0)throw new IOException($"Game core transport error ({status})");return result.Read();}}}
 public void Dispose(){_handle.Dispose();_gate.Dispose();}
}
