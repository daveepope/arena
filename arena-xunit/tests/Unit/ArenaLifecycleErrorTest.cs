using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Lifecycle;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaLifecycleErrorTest
{
    [Fact]
    public void From_AlreadyLifecycleError_ReturnsSameInstance()
    {
        var error = new ArenaLifecycleError("faulted", null);

        Assert.Same(error, ArenaLifecycleError.From(error));
    }

    [Fact]
    public void From_ErrorWithoutStateDocument_ReturnsSameInstance()
    {
        var error = new ArenaBindingError("plain failure");

        Assert.Same(error, ArenaLifecycleError.From(error));
    }

    [Fact]
    public void From_ErrorWithUnparseableStateDocument_ReturnsSameInstance()
    {
        var error = new ArenaBindingError("broken doc", "{broken");

        Assert.Same(error, ArenaLifecycleError.From(error));
    }

    [Fact]
    public void From_ErrorWithValidStateDocument_ReturnsLifecycleErrorCarryingState()
    {
        var error = new ArenaBindingError("arena 'orders' is arena_faulted", ArenaStateTest.FixtureStateJson);

        var converted = ArenaLifecycleError.From(error);

        var lifecycle = Assert.IsType<ArenaLifecycleError>(converted);
        Assert.Equal("arena 'orders' is arena_faulted", lifecycle.Message);
        Assert.NotNull(lifecycle.State);
        Assert.Equal("orders", lifecycle.State!.Id);
        Assert.True(lifecycle.State!.IsFaulted());
    }

}
