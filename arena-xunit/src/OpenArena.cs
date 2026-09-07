using System;
using System.Collections.Generic;
using System.Threading;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Lifecycle;
using ArenaDotnet.Xunit.Playbook;

namespace ArenaDotnet.Xunit;

public sealed class OpenArena : IDisposable
{
    private readonly ArenaHandle _handle;
    private readonly ulong _logToken;
    private readonly ulong _lifecycleObserverToken;
    private readonly ArenaLogRouting _routing;
    private readonly Match _match;
    private readonly Dictionary<Type, ActivePlaybook> _sessionPlaybooks;
    private readonly Dictionary<Type, Playbook.IPlaybook> _registeredPlaybooks;
    private readonly Dictionary<Type, bool> _playbookExecOnStart;
    private int _disposed;

    internal OpenArena(
        IntPtr handle,
        ulong logToken,
        ulong lifecycleObserverToken,
        ArenaLogRouting routing,
        Match match,
        Dictionary<Type, ActivePlaybook> sessionPlaybooks)
    {
        _handle = new ArenaHandle(handle);
        _logToken = logToken;
        _lifecycleObserverToken = lifecycleObserverToken;
        _routing = routing;
        _match = match;
        _sessionPlaybooks = sessionPlaybooks;
        _registeredPlaybooks = new Dictionary<Type, Playbook.IPlaybook>();
        _playbookExecOnStart = new Dictionary<Type, bool>();
        foreach (var reg in match.Playbooks)
        {
            _registeredPlaybooks[reg.Playbook.GetType()] = reg.Playbook;
            _playbookExecOnStart[reg.Playbook.GetType()] = reg.ExecOnDependencyStart;
        }
        ArenaShutdown.Track(this);
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

    public ArenaState State()
    {
        ThrowIfDisposed();
        return ArenaState.Parse(ArenaBindings.StateJson(Handle));
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
        if (Interlocked.Exchange(ref _disposed, 1) != 0)
            return;
        ArenaShutdown.Untrack(this);

        List<Exception>? errors = null;
        foreach (var pb in _sessionPlaybooks.Values)
        {
            try
            {
                pb.Dispose();
            }
            catch (Exception ex)
            {
                (errors ??= new List<Exception>()).Add(ex);
            }
        }
        _sessionPlaybooks.Clear();

        string? stateDocument = null;
        IntPtr raw = _handle.DangerousGetHandle();
        _handle.SetHandleAsInvalid();
        try
        {
            if (raw != IntPtr.Zero)
                stateDocument = ArenaBindings.CloseArena(raw);
        }
        catch (ArenaBindingError ex)
        {
            (errors ??= new List<Exception>()).Add(ArenaLifecycleError.From(ex));
        }
        finally
        {
            ArenaLifecycleObservers.Unregister(_lifecycleObserverToken);
            ArenaLogTarget.Unregister(_logToken);
        }
        if (stateDocument != null)
            LifecycleLog.LogClosingSummaryDocument(stateDocument, _routing);

        if (errors == null)
            return;
        if (errors.Count == 1)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(errors[0]).Throw();
        throw new AggregateException("arena close completed with failures", errors);
    }

    internal void ThrowIfDisposed()
    {
        if (Volatile.Read(ref _disposed) != 0)
            throw new ObjectDisposedException(nameof(OpenArena));
    }
}
