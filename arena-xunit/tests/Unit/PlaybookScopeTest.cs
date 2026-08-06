using System;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class PlaybookScopeTest : IClassFixture<PlaybookScopeTest.Fixture>
{
    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("playbook-scope-match").Build();
    }

    private static OpenArena? StaticArena;

    public PlaybookScopeTest(Fixture fixture)
    {
        StaticArena = fixture.Arena;
    }

    private class NoAttributeTestClass
    {
        public static OpenArena? Arena => StaticArena;
        public void SomeTest() { }
    }

    [Playbook(typeof(object))]
    private class ClassLevelAttributeTestClass
    {
        public static OpenArena? Arena => StaticArena;
        public void SomeTest() { }
    }

    private class MethodLevelAttributeTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(object))]
        public void SomeTest() { }
    }

    private class NoStaticArenaTestClass
    {
        [Playbook(typeof(object))]
        public void SomeTest() { }
    }

    [Fact]
    public void BeforeTest_NoPlaybookAttributes_ReturnsWithoutThrowing()
    {
        var method = typeof(NoAttributeTestClass).GetMethod(nameof(NoAttributeTestClass.SomeTest))!;
        PlaybookScope.BeforeTest(method, typeof(NoAttributeTestClass));
        PlaybookScope.AfterTest(method, typeof(NoAttributeTestClass));
    }

    [Fact]
    public void BeforeTest_NoStaticArenaProperty_ReturnsWithoutThrowing()
    {
        var method = typeof(NoStaticArenaTestClass).GetMethod(nameof(NoStaticArenaTestClass.SomeTest))!;
        PlaybookScope.BeforeTest(method, typeof(NoStaticArenaTestClass));
    }

    [Fact]
    public void BeforeTest_MethodLevelAttributeUnregisteredPlaybook_ThrowsInvalidOperationException()
    {
        var method = typeof(MethodLevelAttributeTestClass).GetMethod(nameof(MethodLevelAttributeTestClass.SomeTest))!;
        var ex = Assert.Throws<InvalidOperationException>(
            () => PlaybookScope.BeforeTest(method, typeof(MethodLevelAttributeTestClass)));
        Assert.Contains("no playbook of type", ex.Message);
    }

    [Fact]
    public void BeforeTest_ClassLevelAttributeUnregisteredPlaybook_ThrowsInvalidOperationException()
    {
        var method = typeof(ClassLevelAttributeTestClass).GetMethod(nameof(ClassLevelAttributeTestClass.SomeTest))!;
        Assert.Throws<InvalidOperationException>(
            () => PlaybookScope.BeforeTest(method, typeof(ClassLevelAttributeTestClass)));
    }

    [Fact]
    public void AfterTest_WithoutPriorBeforeTest_ReturnsWithoutThrowing()
    {
        var method = typeof(NoAttributeTestClass).GetMethod(nameof(NoAttributeTestClass.SomeTest))!;
        PlaybookScope.AfterTest(method, typeof(NoAttributeTestClass));
    }
}
