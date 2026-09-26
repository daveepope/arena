using System;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaNativeLibSymbolsTest
{
    [Fact]
    public void ArenaNativeLib_FirstCall_ResolvesEveryDeclaredExport()
    {
        var caught = Record.Exception(() => ArenaNativeLib.arena_free_string(IntPtr.Zero));

        Assert.Null(caught);
    }

    [Fact]
    public void ArenaNativeLib_ArenaCloseNullHandle_ReturnsInvalidArgument()
    {
        var status = ArenaNativeLib.arena_close(IntPtr.Zero, out var errOut, out var stateOut);

        Assert.Equal(1, status);
        Assert.NotEqual(IntPtr.Zero, errOut);
        Assert.Equal(IntPtr.Zero, stateOut);
        ArenaNativeLib.arena_free_string(errOut);
    }
}
