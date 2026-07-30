using System;
using System.Collections.Generic;
using System.Linq;
using ArenaXunit.Ffi;
using ArenaXunit.Match;
using ArenaXunit.Playbook;
using ArenaXunit.Support;
using Microsoft.Extensions.Logging;

namespace ArenaXunit;

public sealed class ClosedArena
{
    private readonly string _name;
    private readonly Match _match;
    private readonly ArenaLogLevel _logLevel;
    private readonly ILogger? _logger;
    private readonly List<string>? _logDependencyIds;
    private readonly List<string>? _logComponentIds;

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

    public ClosedArena(string name, Match match, ArenaLogLevel logLevel, ILogger? logger,
        List<string>? logDependencyIds, List<string>? logComponentIds)
    {
        _name = name;
        _match = match;
        _logLevel = logLevel;
        _logger = logger;
        _logDependencyIds = logDependencyIds;
        _logComponentIds = logComponentIds;
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

        ulong logToken = _logger != null
            ? ArenaLogTarget.RegisterForLogger(_logger, _logLevel)
            : ArenaLogTarget.RegisterForLogger(CreateDefaultLogger(), _logLevel);

        try
        {
            var handle = ArenaBindings.OpenArena(_name, json, _logLevel);
            var playbooks = RunExecOnStartPlaybooks(handle);
            return System.Threading.Tasks.Task.FromResult(new OpenArena(handle, logToken, _match, playbooks));
        }
        catch
        {
            ArenaLogTarget.Unregister(logToken);
            throw;
        }
    }

    private Dictionary<Type, ActivePlaybook> RunExecOnStartPlaybooks(IntPtr handle)
    {
        var result = new Dictionary<Type, ActivePlaybook>();
        foreach (var registered in _match.Playbooks)
        {
            if (registered.ExecOnDependencyStart)
            {
                var active = ArenaBindings.MatchPlaybookRun(handle, registered.Playbook.Identifier);
                result[registered.Playbook.GetType()] = active;
            }
        }
        return result;
    }

    private static ILogger CreateDefaultLogger()
    {
        return new ConsoleLogger();
    }

    private sealed class ConsoleLogger : ILogger
    {
        public IDisposable? BeginScope<TState>(TState state) => null;
        public bool IsEnabled(LogLevel level) => true;
        public void Log<TState>(LogLevel level, EventId eventId, TState state,
            Exception? exception, Func<TState, Exception?, string> formatter)
        {
            var msg = formatter(state, exception);
            Console.WriteLine($"{level}: {msg}");
        }
    }
}
