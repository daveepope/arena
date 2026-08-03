using System.Collections.Generic;

namespace ArenaXunit.Component;

public interface IArenaReadinessCheck
{
}

public sealed class HttpReadinessCheck : IArenaReadinessCheck
{
    private HttpReadinessCheck()
    {
    }

    public static HttpReadinessCheck Create() => new();
}

internal sealed class ReadinessCheckEntry
{
    public ReadinessCheckEntry(IArenaReadinessCheck check, string target, long timeoutMs)
    {
        Check = check;
        Target = target;
        TimeoutMs = timeoutMs;
    }

    public IArenaReadinessCheck Check { get; }
    public string Target { get; }
    public long TimeoutMs { get; }
}

internal static class ReadinessCheckWireFormat
{
    public const long DefaultTimeoutMs = 10_000;

    public static List<object>? Build(IReadOnlyList<ReadinessCheckEntry> entries)
    {
        var result = new List<object>();
        foreach (var entry in entries)
        {
            if (entry.Check is HttpReadinessCheck)
            {
                result.Add(new
                {
                    kind = "http",
                    target = entry.Target,
                    timeout_ms = entry.TimeoutMs,
                });
            }
        }
        return result.Count > 0 ? result : null;
    }
}
