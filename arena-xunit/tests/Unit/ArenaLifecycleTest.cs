using System;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaLifecycleTest : IClassFixture<ArenaLifecycleTest.Fixture>
{
    private readonly OpenArena _arena;

    public ArenaLifecycleTest(Fixture fixture)
    {
        _arena = fixture.Arena;
    }

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("lifecycle-empty-match").Build();
    }

    [Fact]
    public void OpenAsync_EmptyMatch_OpensAndClosesSuccessfully()
    {
        Assert.NotNull(_arena);
    }

    [Fact]
    public void GetPlaybook_NoPlaybooksRegistered_ReturnsNull()
    {
        var playbook = _arena.GetPlaybook(typeof(object));
        Assert.Null(playbook);
    }

    [Fact]
    public void GetSessionPlaybook_NoPlaybooksRegistered_ReturnsNull()
    {
        var playbook = _arena.GetSessionPlaybook(typeof(object));
        Assert.Null(playbook);
    }

    [Fact]
    public void PlaybookExecOnDependencyStart_UnknownType_ReturnsFalse()
    {
        Assert.False(_arena.PlaybookExecOnDependencyStart(typeof(object)));
    }

    [Fact]
    public void SoftReset_UnknownDependency_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => _arena.SoftReset("does-not-exist"));
    }

    [Fact]
    public void HardReset_UnknownDependency_ThrowsArenaBindingError()
    {
        Assert.Throws<ArenaBindingError>(() => _arena.HardReset("does-not-exist"));
    }

    [Fact]
    public void GetPlaybook_AfterDispose_ThrowsObjectDisposedException()
    {
        var closedArena = new ClosedArena("dispose-test", new MatchBuilder("dispose-test-match").Build());
        var openArena = closedArena.OpenAsync().Result;
        ((IDisposable)openArena).Dispose();
        Assert.Throws<ObjectDisposedException>(() => openArena.GetPlaybook(typeof(object)));
    }

    [Fact]
    public void Dispose_CalledMultipleTimes_DoesNotThrow()
    {
        var closedArena = new ClosedArena("dispose-twice-test", new MatchBuilder("dispose-twice-match").Build());
        var openArena = closedArena.OpenAsync().Result;
        ((IDisposable)openArena).Dispose();
        ((IDisposable)openArena).Dispose();
    }
}
