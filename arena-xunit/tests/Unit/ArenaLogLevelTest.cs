using ArenaXunit.Ffi;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaLogLevelTest
{
    [Fact]
    public void Trace_HasValueZero()
    {
        Assert.Equal(0, (int)ArenaLogLevel.Trace);
    }

    [Fact]
    public void Debug_HasValueOne()
    {
        Assert.Equal(1, (int)ArenaLogLevel.Debug);
    }

    [Fact]
    public void Info_HasValueTwo()
    {
        Assert.Equal(2, (int)ArenaLogLevel.Info);
    }

    [Fact]
    public void Warn_HasValueThree()
    {
        Assert.Equal(3, (int)ArenaLogLevel.Warn);
    }

    [Fact]
    public void Error_HasValueFour()
    {
        Assert.Equal(4, (int)ArenaLogLevel.Error);
    }

    [Fact]
    public void Critical_HasValueFive()
    {
        Assert.Equal(5, (int)ArenaLogLevel.Critical);
    }
}
