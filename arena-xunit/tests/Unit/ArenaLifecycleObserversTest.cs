using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaLifecycleObserversTest
{
    [Fact]
    public void Unregister_UnknownToken_DoesNotThrow()
    {
        ArenaLifecycleObservers.Unregister(0);
        ArenaLifecycleObservers.Unregister(ulong.MaxValue);
    }

    [Fact]
    public void Register_NullCallback_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => ArenaLifecycleObservers.Register(null!));
    }
}
