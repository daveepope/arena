using System;
using System.Threading.Tasks;
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
        _safeHandle.Dispose();
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

}
