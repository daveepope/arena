using ArenaXunit.Ffi;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaLogLevelTest
{
    [Fact]
    public void trace_has_value_zero()
    {
        Assert.Equal(0, (int)ArenaLogLevel.Trace);
    }

    [Fact]
    public void debug_has_value_one()
    {
        Assert.Equal(1, (int)ArenaLogLevel.Debug);
    }

    [Fact]
    public void info_has_value_two()
    {
        Assert.Equal(2, (int)ArenaLogLevel.Info);
    }

    [Fact]
    public void warn_has_value_three()
    {
        Assert.Equal(3, (int)ArenaLogLevel.Warn);
    }

    [Fact]
    public void error_has_value_four()
    {
        Assert.Equal(4, (int)ArenaLogLevel.Error);
    }

    [Fact]
    public void critical_has_value_five()
    {
        Assert.Equal(5, (int)ArenaLogLevel.Critical);
    }
}
