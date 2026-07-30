using ArenaXunit.Ffi;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaBindingErrorTest
{
    [Fact]
    public void Constructor_WithMessage_SetsMessage()
    {
        var ex = new ArenaBindingError("test error");
        Assert.Equal("test error", ex.Message);
    }

    [Fact]
    public void Constructor_WithInner_SetsInnerException()
    {
        var inner = new System.Exception("inner");
        var ex = new ArenaBindingError("test", inner);
        Assert.Same(inner, ex.InnerException);
    }

    [Fact]
    public void Inherits_FromException()
    {
        var ex = new ArenaBindingError("test");
        Assert.IsAssignableFrom<System.Exception>(ex);
    }
}
