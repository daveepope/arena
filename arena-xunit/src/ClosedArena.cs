using System;
using System.Collections.Generic;
using System.Linq;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Lifecycle;
using ArenaDotnet.Xunit.Playbook;
using ArenaDotnet.Xunit.Support;
using Microsoft.Extensions.Logging;

namespace ArenaDotnet.Xunit;

public sealed class ClosedArena
{
    private const int DefaultDashboardPort = 4873;

    private readonly string _name;
    private readonly Match _match;
    private readonly ArenaLogLevel _logLevel;
    private readonly ILogger? _logger;
    private readonly ILoggerFactory? _loggerFactory;
    private readonly List<string>? _logDependencyIds;
    private readonly List<string>? _logComponentIds;
    private readonly bool _lifecycleDashboard;
    private readonly string? _dashboardUrl;
    private readonly int _dashboardPort;
    private readonly string? _dashboardSearchDir;

    public ClosedArena(string name, Match match)
        : this(name, match, ArenaLogLevel.Info, null, null, null)
    {
    }

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel)
        : this(name, match, logLevel, null, null, null)
    {
    }

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILogger? logger)
        : this(name, match, logLevel, logger, null, null)
    {
    }

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILoggerFactory? loggerFactory)
        : this(name, match, logLevel, null, loggerFactory, null, null)
    {
    }

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILogger? logger,
        List<string>? logDependencyIds, List<string>? logComponentIds)
        : this(name, match, logLevel, logger, null, logDependencyIds, logComponentIds)
    {
    }

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILogger? logger,
        ILoggerFactory? loggerFactory, List<string>? logDependencyIds, List<string>? logComponentIds)
        : this(name, match, logLevel, logger, loggerFactory, logDependencyIds, logComponentIds,
            false, null, DefaultDashboardPort, null)
    {
    }

    private ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILogger? logger,
        ILoggerFactory? loggerFactory, List<string>? logDependencyIds, List<string>? logComponentIds,
        bool lifecycleDashboard, string? dashboardUrl, int dashboardPort, string? dashboardSearchDir)
    {
        _name = name;
        _match = match;
        _logLevel = logLevel;
        _logger = logger;
        _loggerFactory = loggerFactory;
        _logDependencyIds = logDependencyIds;
        _logComponentIds = logComponentIds;
        _lifecycleDashboard = lifecycleDashboard;
        _dashboardUrl = dashboardUrl;
        _dashboardPort = dashboardPort;
        _dashboardSearchDir = dashboardSearchDir;
    }

    public ClosedArena WithLifecycleDashboard() =>
        WithLifecycleDashboardInternal(null, DefaultDashboardPort, null);

    public ClosedArena WithLifecycleDashboard(string url) =>
        WithLifecycleDashboardInternal(url, DefaultDashboardPort, null);

    public ClosedArena WithLifecycleDashboard(int port, string? searchDir) =>
        WithLifecycleDashboardInternal(null, port, searchDir);

    private ClosedArena WithLifecycleDashboardInternal(string? url, int port, string? searchDir) =>
        new(_name, _match, _logLevel, _logger, _loggerFactory, _logDependencyIds, _logComponentIds,
            true, url, port, searchDir);

    private ulong RegisterDashboard(ArenaLogRouting routing)
    {
        if (!_lifecycleDashboard)
            return 0;
        try
        {
            var searchDir = _dashboardSearchDir ?? Ffi.ArenaPaths.ResolveDashboardSearchDirectory();
            var url = _dashboardUrl ?? ArenaBindings.DashboardEnsureRunning(_dashboardPort, searchDir);
            return ArenaBindings.RegisterDashboardObserver(url);
        }
        catch (ArenaBindingError e)
        {
            routing.LoggerFor(ArenaLogRouting.RootLoggerName)
                .LogWarning(e, "lifecycle dashboard unavailable, continuing without it");
            return 0;
        }
    }

    public System.Threading.Tasks.Task<OpenArena> OpenAsync()
    {
        var json = _match.ForFfi();
        ArenaBindings.SetDispatcherDependencyAllowJson(
            _logDependencyIds != null && _logDependencyIds.Count > 0
                ? ArenaJson.Serialize(_logDependencyIds) : null);
        ArenaBindings.SetDispatcherComponentAllowJson(
            _logComponentIds != null && _logComponentIds.Count > 0
                ? ArenaJson.Serialize(_logComponentIds) : null);

        var routing = _loggerFactory != null
            ? new ArenaLogRouting(_loggerFactory)
            : new ArenaLogRouting(_logger ?? CreateDefaultLogger());
        ulong logToken = ArenaLogTarget.Register(routing);
        ulong observerToken;
        try
        {
            observerToken = ArenaLifecycleObservers.Register(
                document => LifecycleLog.LogTransitionDocument(_name, routing, document));
        }
        catch
        {
            ArenaLogTarget.Unregister(logToken);
            throw;
        }

        ulong dashboardToken = RegisterDashboard(routing);

        ArenaShutdown.EnsureHooksRegistered();

        IntPtr handle;
        try
        {
            handle = ArenaBindings.OpenArena(_name, json, _logLevel);
        }
        catch (ArenaBindingError e)
        {
            ArenaBindings.RemoveLifecycleObserver(dashboardToken);
            ArenaLifecycleObservers.Unregister(observerToken);
            ArenaLogTarget.Unregister(logToken);
            throw ArenaLifecycleError.From(e);
        }
        catch
        {
            ArenaBindings.RemoveLifecycleObserver(dashboardToken);
            ArenaLifecycleObservers.Unregister(observerToken);
            ArenaLogTarget.Unregister(logToken);
            throw;
        }

        Dictionary<Type, ActivePlaybook> playbooks;
        try
        {
            playbooks = RunExecOnStartPlaybooks(handle);
        }
        catch
        {
            CloseArenaQuietly(handle);
            ArenaBindings.RemoveLifecycleObserver(dashboardToken);
            ArenaLifecycleObservers.Unregister(observerToken);
            ArenaLogTarget.Unregister(logToken);
            throw;
        }

        return System.Threading.Tasks.Task.FromResult(
            new OpenArena(handle, logToken, observerToken, dashboardToken, routing, _match, playbooks));
    }

    private static void CloseArenaQuietly(IntPtr handle)
    {
        try
        {
            ArenaBindings.CloseArena(handle);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"arena: teardown after a failed open failed: {ex.Message}");
        }
    }

    private Dictionary<Type, ActivePlaybook> RunExecOnStartPlaybooks(IntPtr handle)
    {
        var result = new Dictionary<Type, ActivePlaybook>();
        try
        {
            foreach (var registered in _match.Playbooks)
            {
                if (registered.ExecOnDependencyStart)
                {
                    var playbookHandle = ArenaBindings.MatchPlaybookRun(handle, registered.Playbook.Identifier);
                    result[registered.Playbook.GetType()] = WrapActivePlaybook(registered.Playbook, playbookHandle);
                }
            }
        }
        catch
        {
            foreach (var started in result.Values)
            {
                try
                {
                    started.Dispose();
                }
                catch
                {
                }
            }
            throw;
        }
        return result;
    }

    private static ActivePlaybook WrapActivePlaybook(Playbook.IPlaybook playbook, IntPtr handle)
    {
        if (playbook is ManagedPlaybook managed)
            return managed.WrapHandle(handle);
        throw new InvalidOperationException($"unsupported playbook type: {playbook.GetType()}");
    }

    private static ILogger CreateDefaultLogger()
    {
        return new ConsoleLogger();
    }

    private sealed class ConsoleLogger : ILogger
    {
        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;
        public bool IsEnabled(LogLevel level) => true;
        public void Log<TState>(LogLevel level, EventId eventId, TState state,
            Exception? exception, Func<TState, Exception?, string> formatter)
        {
            var msg = formatter(state, exception);
            Console.WriteLine($"{level}: {msg}");
        }
    }
}
