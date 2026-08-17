using System;
using System.Reflection;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ClosedArenaDefaultLoggerTest
{
    [Fact]
    public void BeginScope_AnyState_ReturnsNull()
    {
        var loggerType = typeof(ClosedArena).GetNestedType("ConsoleLogger", BindingFlags.NonPublic);
        Assert.NotNull(loggerType);
        var logger = (ILogger)Activator.CreateInstance(loggerType!)!;

        var scope = logger.BeginScope("some-state");

        Assert.Null(scope);
    }
}
