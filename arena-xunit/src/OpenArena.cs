using System;
using System.Collections.Generic;
using ArenaXunit.Ffi;
using ArenaXunit.Playbook;

namespace ArenaXunit;

public sealed class OpenArena : IDisposable
{
    private readonly ArenaHandle _handle;
    private readonly ulong _logToken;
    private readonly Match _match;
    private readonly Dictionary<Type, ActivePlaybook> _sessionPlaybooks;
    private readonly Dictionary<Type, Playbook.IPlaybook> _registeredPlaybooks;
    private readonly Dictionary<Type, bool> _playbookExecOnStart;
    private bool _disposed;

    internal OpenArena(IntPtr handle, ulong logToken, Match match, Dictionary<Type, ActivePlaybook> sessionPlaybooks)
    {
        _handle = new ArenaHandle(handle);
        _logToken = logToken;
        _match = match;
        _sessionPlaybooks = sessionPlaybooks;
        _registeredPlaybooks = new Dictionary<Type, Playbook.IPlaybook>();
        _playbookExecOnStart = new Dictionary<Type, bool>();
        foreach (var reg in match.Playbooks)
        {
            _registeredPlaybooks[reg.Playbook.GetType()] = reg.Playbook;
            _playbookExecOnStart[reg.Playbook.GetType()] = reg.ExecOnDependencyStart;
        }
    }

    internal IntPtr Handle => _handle.DangerousGetHandle();

    public void SoftReset(string dependencyIdentifier)
    {
        ThrowIfDisposed();
        ArenaBindings.SoftReset(Handle, dependencyIdentifier);
    }

    public void HardReset(string dependencyIdentifier)
    {
        ThrowIfDisposed();
        ArenaBindings.HardReset(Handle, dependencyIdentifier);
    }

    public Playbook.IPlaybook? GetPlaybook(Type playbookType)
    {
        ThrowIfDisposed();
        _registeredPlaybooks.TryGetValue(playbookType, out var pb);
        return pb;
    }

    public T? GetPlaybook<T>() where T : class, Playbook.IPlaybook
    {
        return GetPlaybook(typeof(T)) as T;
    }

    public ActivePlaybook? GetSessionPlaybook(Type playbookType)
    {
        ThrowIfDisposed();
        _sessionPlaybooks.TryGetValue(playbookType, out var pb);
        return pb;
    }

    public T? GetSessionPlaybook<T>() where T : ActivePlaybook
    {
        return GetSessionPlaybook(typeof(T)) as T;
    }

    public bool PlaybookExecOnDependencyStart(Type playbookType)
    {
        ThrowIfDisposed();
        _playbookExecOnStart.TryGetValue(playbookType, out var val);
        return val;
    }

    public bool PlaybookExecOnDependencyStart<T>()
    {
        return PlaybookExecOnDependencyStart(typeof(T));
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;

        foreach (var pb in _sessionPlaybooks.Values)
            pb.Dispose();
        _sessionPlaybooks.Clear();

        try
        {
            _handle.Dispose();
        }
        finally
        {
            ArenaLogTarget.Unregister(_logToken);
        }
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(OpenArena));
    }
}
