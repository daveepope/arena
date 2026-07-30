using System;
using ArenaXunit.Ffi;

namespace ArenaXunit.Playbook;

public sealed class ActivePlaybook : IDisposable
{
    private readonly IntPtr _handle;
    private bool _disposed;

    internal ActivePlaybook(IntPtr handle)
    {
        _handle = handle;
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;
        try
        {
            ArenaBindings.ActivePlaybookDrop(_handle);
        }
        catch
        {
        }
    }
}
