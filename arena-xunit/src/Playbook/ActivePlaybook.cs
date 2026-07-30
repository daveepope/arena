using System;
using System.Threading.Tasks;
using ArenaXunit.Ffi;

namespace ArenaXunit.Playbook;

public class ActivePlaybook : IDisposable, System.IAsyncDisposable
{
    protected readonly IntPtr _handle;
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

    public Task VerifyAtLeast(string path, int minCount)
    {
        var http = this as ActiveHttpPlaybook;
        if (http == null)
        {
            throw new InvalidOperationException("VerifyAtLeast is only supported on HTTP playbooks");
        }
        http.VerifyAtLeast("POST", path, minCount);
        return Task.CompletedTask;
    }

    public System.Threading.Tasks.ValueTask DisposeAsync()
    {
        Dispose();
        return default;
    }
}
