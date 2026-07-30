using System;
using System.Collections.Generic;
using ArenaXunit.Ffi;
using ArenaXunit.Match;
using ArenaXunit.Playbook;
using ArenaXunit.Support;
using Microsoft.Extensions.Logging;

namespace ArenaXunit;

public sealed class OpenArena
{
    private readonly IntPtr _handle;
    private readonly ulong _logToken;
    private readonly Match _match;
    private readonly Dictionary<Type, ActivePlaybook> _sessionPlaybooks;
    private bool _disposed;

    internal OpenArena(IntPtr handle, ulong logToken, Match match, Dictionary<Type, ActivePlaybook> sessionPlaybooks)
    {
        _handle = handle;
        _logToken = logToken;
        _match = match;
        _sessionPlaybooks = sessionPlaybooks;
    }

    public void SoftReset(string dependencyIdentifier)
    {
        ThrowIfDisposed();
        ArenaBindings.SoftReset(_handle, dependencyIdentifier);
    }

    public void HardReset(string dependencyIdentifier)
    {
        ThrowIfDisposed();
        ArenaBindings.HardReset(_handle, dependencyIdentifier);
    }

    public ActivePlaybook? GetPlaybook(Type playbookType)
    {
        ThrowIfDisposed();
        _sessionPlaybooks.TryGetValue(playbookType, out var pb);
        return pb;
    }

    public bool PlaybookExecOnDependencyStart(Type playbookType)
    {
        ThrowIfDisposed();
        foreach (var reg in _match.Playbooks)
        {
            if (reg.Playbook.GetType() == playbookType)
                return reg.ExecOnDependencyStart;
        }
        return false;
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;

        foreach (var pb in _sessionPlaybooks.Values)
        {
            pb.Dispose();
        }
        _sessionPlaybooks.Clear();

        try
        {
            ArenaBindings.CloseArena(_handle);
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
