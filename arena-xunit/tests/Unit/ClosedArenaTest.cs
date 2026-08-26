using System.Threading.Tasks;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ClosedArenaTest
{
    [Fact]
    public async Task OpenAsync_ComponentFailsToStart_RethrowsAndUnregistersLogTarget()
    {
        var component = new ExecutableComponentBuilder("broken")
            .WithExecutablePath("closed-arena-test-does-not-exist-binary")
            .Build();
        var match = new MatchBuilder("closed-arena-open-failure-probe")
            .AddComponent(component)
            .Build();
        var closedArena = new ClosedArena("closed-arena-open-failure-probe", match);

        await Assert.ThrowsAsync<ArenaBindingError>(() => closedArena.OpenAsync());
    }
}
