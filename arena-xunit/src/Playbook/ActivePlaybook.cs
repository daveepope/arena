using System;
using ArenaXunit.Ffi;

namespace ArenaXunit.Playbook;

public class ActivePlaybook : IDisposable
{
    protected readonly IntPtr _handle;
    private readonly ActivePlaybookHandle _safeHandle;
    private bool _disposed;

    internal ActivePlaybook(IntPtr handle)
    {
        _handle = handle;
        _safeHandle = new ActivePlaybookHandle(handle);
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;

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
