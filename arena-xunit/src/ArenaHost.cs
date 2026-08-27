using ArenaDotnet.Xunit.Ffi;

namespace ArenaDotnet.Xunit;

public static class ArenaHost
{
    public static int FindAvailablePort(int rangeStart, int rangeEnd, PortSearchStrategy strategy = PortSearchStrategy.Random)
    {
        return ArenaBindings.FindAvailablePort(rangeStart, rangeEnd, strategy);
    }
}
