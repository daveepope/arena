using ArenaXunit.Ffi;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaBindingErrorTest
{
    [Fact]
    public void constructor_with_message_sets_message()
    {
        var ex = new ArenaBindingError("test error");
        Assert.Equal("test error", ex.Message);
    }

    [Fact]
    public void constructor_with_inner_sets_inner_exception()
    {
        var inner = new System.Exception("inner");
        var ex = new ArenaBindingError("test", inner);
        Assert.Same(inner, ex.InnerException);
    }

    [Fact]
    public void inherits_from_exception()
    {
        var ex = new ArenaBindingError("test");
        Assert.IsAssignableFrom<System.Exception>(ex);
    }
}
