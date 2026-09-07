using System;
using System.Threading;
using ArenaDotnet.Xunit.Ffi;

namespace ArenaDotnet.Xunit.Playbook;

public class ActivePlaybook : IDisposable
{
    protected readonly IntPtr _handle;
    private readonly ActivePlaybookHandle _safeHandle;
    private int _disposed;

    internal ActivePlaybook(IntPtr handle)
    {
        _handle = handle;
        _safeHandle = new ActivePlaybookHandle(handle);
    }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0)
            return;

        if (_safeHandle.IsInvalid)
        {
            _safeHandle.SetHandleAsInvalid();
            return;
        }

        try
        {
            ArenaBindings.ActivePlaybookDrop(_handle);
        }
        finally
        {
            _safeHandle.SetHandleAsInvalid();
        }
    }
}
