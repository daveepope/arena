using ArenaXunit.Ffi;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaLogLevelTest
{
    [Fact]
    public void Trace_Value_IsZero()
    {
        Assert.Equal(0, (int)ArenaLogLevel.Trace);
    }

    [Fact]
    public void Debug_Value_IsOne()
    {
        Assert.Equal(1, (int)ArenaLogLevel.Debug);
    }

    [Fact]
    public void Info_Value_IsTwo()
    {
        Assert.Equal(2, (int)ArenaLogLevel.Info);
    }

    [Fact]
    public void Warn_Value_IsThree()
    {
        Assert.Equal(3, (int)ArenaLogLevel.Warn);
    }

    [Fact]
    public void Error_Value_IsFour()
    {
        Assert.Equal(4, (int)ArenaLogLevel.Error);
    }

    [Fact]
    public void Critical_Value_IsFive()
    {
        Assert.Equal(5, (int)ArenaLogLevel.Critical);
    }
}
