using System;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Ffi;

namespace ArenaExamples.Test.Shared;

public static class EphemeralTestRuntime
{
    private const int EphemeralPortRangeStart = 20600;
    private const int EphemeralPortRangeEnd = 20900;
    private static readonly string Suffix = Guid.NewGuid().ToString("N")[..8];

    public static int AllocatePort()
    {
        return ArenaHost.FindAvailablePort(EphemeralPortRangeStart, EphemeralPortRangeEnd, PortSearchStrategy.Random);
    }

    public static string NetworkName => $"arena-example-api-network-{Suffix}";

    public static string RandomToken(int length) => Guid.NewGuid().ToString("N")[..length];
}
