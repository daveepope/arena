using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Ffi;

namespace ArenaExamples.Test.Shared;

public static class EphemeralTestRuntime
{
    private const int EphemeralPortRangeStart = 20600;
    private const int EphemeralPortRangeEnd = 20900;
    private static readonly string Suffix = Guid.NewGuid().ToString("N")[..8];

    private static readonly IReadOnlyDictionary<string, (int Start, int End)> TargetPortRanges =
        new Dictionary<string, (int Start, int End)>
        {
            ["//examples:example-aspnet-xunit-component-test"] = (20600, 20750),
            ["//examples:example-aspnet-xunit-chained-component-test"] = (20750, 20900),
        };

    internal static (int Start, int End) PortRangeForTarget(string? target)
    {
        if (target != null && TargetPortRanges.TryGetValue(target, out var range))
        {
            return range;
        }

        return (EphemeralPortRangeStart, EphemeralPortRangeEnd);
    }

    public static int AllocatePort()
    {
        var (start, end) = PortRangeForTarget(Environment.GetEnvironmentVariable("TEST_TARGET"));
        return ArenaHost.FindAvailablePort(start, end, PortSearchStrategy.Random);
    }

    public static string NetworkName => $"arena-example-api-network-{Suffix}";

    public static string RandomToken(int length) => Guid.NewGuid().ToString("N")[..length];
}
