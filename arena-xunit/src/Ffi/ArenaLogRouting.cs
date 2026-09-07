using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;

namespace ArenaDotnet.Xunit.Ffi;

internal sealed class ArenaLogRouting
{
    internal const string RootLoggerName = "arena";

    private readonly ILogger? _logger;
    private readonly ILoggerFactory? _loggerFactory;
    private readonly ConcurrentDictionary<string, ILogger> _loggersByName = new();

    public ArenaLogRouting(ILogger logger) => _logger = logger;

    public ArenaLogRouting(ILoggerFactory loggerFactory) => _loggerFactory = loggerFactory;

    public ILogger LoggerFor(string loggerName)
    {
        if (_loggerFactory == null)
            return _logger!;
        var name = string.IsNullOrEmpty(loggerName) ? RootLoggerName : loggerName;
        return _loggersByName.GetOrAdd(name, _loggerFactory.CreateLogger);
    }

    public string MessageFor(string loggerName, string message)
    {
        if (_loggerFactory != null || string.IsNullOrEmpty(loggerName))
            return message;
        return $"{loggerName}  {message}";
    }
}
