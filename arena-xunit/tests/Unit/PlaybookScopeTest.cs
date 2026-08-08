using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Playbook;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class PlaybookScopeTest : IClassFixture<PlaybookScopeTest.Fixture>
{
    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("playbook-scope-match")
            .RegisterPlaybook(new SeedPlaybook("seed-a"), false)
            .RegisterPlaybook(new SeedPlaybook2("seed-b"), false)
            .RegisterPlaybook(new UnresolvableLocalstackPlaybook("managed-a", "no-such-dep-a"), false)
            .RegisterPlaybook(new UnresolvableLocalstackPlaybook2("managed-b", "no-such-dep-b"), false)
            .RegisterPlaybook(new UnresolvableHttpPlaybook("managed-before-http", "no-such-dep-c"), false)
            .Build();
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

    private class UnmanagedOnlyTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(SeedPlaybook))]
        public void SomeTest() { }
    }

    private class ManagedOnlyTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(UnresolvableLocalstackPlaybook))]
        public void SomeTest() { }
    }

    private class MixedManagedAndUnmanagedTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(SeedPlaybook))]
        [Playbook(typeof(UnresolvableLocalstackPlaybook))]
        public void SomeTest() { }
    }

    private class ManagedResilienceTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(UnresolvableLocalstackPlaybook))]
        [Playbook(typeof(UnresolvableLocalstackPlaybook2))]
        public void SomeTest() { }
    }

    private class ManagedHttpActivatesBeforeTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(UnresolvableHttpPlaybook))]
        public void SomeTest() { }
    }

    private class BeforeGroupFailureWithManagedAfterTestClass
    {
        public static OpenArena? Arena => StaticArena;

        [Playbook(typeof(SeedPlaybook))]
        [Playbook(typeof(UnresolvableHttpPlaybook))]
        [Playbook(typeof(UnresolvableLocalstackPlaybook))]
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

    [Fact]
    public void BeforeTest_UnmanagedPlaybook_ActivatesImmediately()
    {
        var method = typeof(UnmanagedOnlyTestClass).GetMethod(nameof(UnmanagedOnlyTestClass.SomeTest))!;
        try
        {
            PlaybookScope.BeforeTest(method, typeof(UnmanagedOnlyTestClass));
            var active = PlaybookScope.GetActive<ActiveLocalstackPlaybook>();
            Assert.NotNull(active);
        }
        finally
        {
            PlaybookScope.AfterTest(method, typeof(UnmanagedOnlyTestClass));
        }
    }

    [Fact]
    public void BeforeTest_ManagedPlaybookDefault_DoesNotActivateBeforeTest()
    {
        var method = typeof(ManagedOnlyTestClass).GetMethod(nameof(ManagedOnlyTestClass.SomeTest))!;
        PlaybookScope.BeforeTest(method, typeof(ManagedOnlyTestClass));
        Assert.Throws<InvalidOperationException>(
            () => PlaybookScope.GetActive<ActiveLocalstackPlaybook>());
        Assert.Throws<ArenaBindingError>(
            () => PlaybookScope.AfterTest(method, typeof(ManagedOnlyTestClass)));
    }

    [Fact]
    public void AfterTest_MixedManagedAndUnmanaged_DisposesUnmanagedThenRunsManaged()
    {
        var method = typeof(MixedManagedAndUnmanagedTestClass).GetMethod(nameof(MixedManagedAndUnmanagedTestClass.SomeTest))!;
        PlaybookScope.BeforeTest(method, typeof(MixedManagedAndUnmanagedTestClass));
        var active = PlaybookScope.GetActive<ActiveLocalstackPlaybook>();
        Assert.NotNull(active);

        Assert.Throws<ArenaBindingError>(
            () => PlaybookScope.AfterTest(method, typeof(MixedManagedAndUnmanagedTestClass)));
    }

    [Fact]
    public void AfterTest_TwoManagedPlaybooksBothFail_AttemptsBothAndThrowsAggregate()
    {
        var method = typeof(ManagedResilienceTestClass).GetMethod(nameof(ManagedResilienceTestClass.SomeTest))!;
        PlaybookScope.BeforeTest(method, typeof(ManagedResilienceTestClass));

        var ex = Assert.Throws<AggregateException>(
            () => PlaybookScope.AfterTest(method, typeof(ManagedResilienceTestClass)));

        Assert.Equal(2, ex.InnerExceptions.Count);
        Assert.Contains(ex.InnerExceptions, e => e.Message.Contains("managed-a"));
        Assert.Contains(ex.InnerExceptions, e => e.Message.Contains("managed-b"));
    }

    [Fact]
    public void BeforeTest_ManagedHttpOverride_ActivatesBeforeTestAndAttempted()
    {
        var method = typeof(ManagedHttpActivatesBeforeTestClass).GetMethod(nameof(ManagedHttpActivatesBeforeTestClass.SomeTest))!;
        var ex = Assert.Throws<ArenaBindingError>(
            () => PlaybookScope.BeforeTest(method, typeof(ManagedHttpActivatesBeforeTestClass)));
        Assert.NotNull(ex);
    }

    [Fact]
    public void BeforeAfterTestAttribute_XunitCoreVersion_PinnedToMajorVersion2()
    {
        var version = typeof(global::Xunit.Sdk.BeforeAfterTestAttribute).Assembly.GetName().Version;
        Assert.Equal(2, version?.Major);
    }

    [Fact]
    public void BeforeTest_BeforeGroupActivationFails_StillRunsManagedAfterGroupCleanup()
    {
        var method = typeof(BeforeGroupFailureWithManagedAfterTestClass)
            .GetMethod(nameof(BeforeGroupFailureWithManagedAfterTestClass.SomeTest))!;

        var ex = Assert.Throws<AggregateException>(
            () => PlaybookScope.BeforeTest(method, typeof(BeforeGroupFailureWithManagedAfterTestClass)));

        Assert.Equal(2, ex.InnerExceptions.Count);
        Assert.Contains(ex.InnerExceptions, e => e.Message.Contains("managed-before-http"));
        Assert.Contains(ex.InnerExceptions, e => e.Message.Contains("managed-a"));

        Assert.Throws<InvalidOperationException>(
            () => PlaybookScope.GetActive<ActiveLocalstackPlaybook>());

        PlaybookScope.AfterTest(method, typeof(BeforeGroupFailureWithManagedAfterTestClass));
    }

    private class SeedPlaybook : UnmanagedPlaybook
    {
        public override string Identifier { get; }

        public SeedPlaybook(string identifier)
        {
            Identifier = identifier;
        }

        public override ActivePlaybook Run(OpenArena arena)
        {
            return new ActiveLocalstackPlaybook(IntPtr.Zero);
        }
    }

    private class SeedPlaybook2 : UnmanagedPlaybook
    {
        public override string Identifier { get; }

        public SeedPlaybook2(string identifier)
        {
            Identifier = identifier;
        }

        public override ActivePlaybook Run(OpenArena arena)
        {
            return new ActiveLocalstackPlaybook(IntPtr.Zero);
        }
    }

    private class UnresolvableLocalstackPlaybook : ManagedLocalstackPlaybook
    {
        public UnresolvableLocalstackPlaybook(string identifier, string dependencyIdentifier)
            : base(identifier, dependencyIdentifier)
        {
        }
    }

    private class UnresolvableLocalstackPlaybook2 : ManagedLocalstackPlaybook
    {
        public UnresolvableLocalstackPlaybook2(string identifier, string dependencyIdentifier)
            : base(identifier, dependencyIdentifier)
        {
        }
    }

    private class UnresolvableHttpPlaybook : ManagedHttpPlaybook
    {
        public UnresolvableHttpPlaybook(string identifier, string dependencyIdentifier)
            : base(identifier, dependencyIdentifier, new List<object>())
        {
        }
    }
}
