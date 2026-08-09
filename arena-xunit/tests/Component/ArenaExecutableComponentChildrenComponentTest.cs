using System;
using System.IO;
using System.Threading;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public class ArenaExecutableComponentChildrenComponentTest
    : IClassFixture<ArenaExecutableComponentChildrenComponentTest.Fixture>
{
    private static readonly string _markerFile =
        Path.Combine(Path.GetTempPath(), $"arena-xunit-children-{Guid.NewGuid()}.txt");

    private readonly Fixture _fixture;

    public ArenaExecutableComponentChildrenComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure()
        {
            var child = new ExecutableComponentBuilder("child")
                .WithExecutablePath("/bin/sh")
                .WithRuntimeArg("flag", "-c")
                .WithRuntimeArg("script", $"echo child >> {_markerFile}")
                .Build();

            var parent = new ExecutableComponentBuilder("parent")
                .WithExecutablePath("/bin/sh")
                .WithRuntimeArg("flag", "-c")
                .WithRuntimeArg("script", $"echo parent >> {_markerFile}")
                .WithChildComponents(new[] { child })
                .Build();

            return new MatchBuilder("children-lifecycle-match")
                .AddComponent(parent)
                .Build();
        }
    }

    [Fact]
    internal void OpenArena_WithChildComponent_StartsBothParentAndChild()
    {
        Assert.NotNull(_fixture.Arena);
        var lines = WaitForMarkerLines(TimeSpan.FromSeconds(5));
        Assert.Contains("child", lines);
        Assert.Contains("parent", lines);
    }

    private static string[] WaitForMarkerLines(TimeSpan timeout)
    {
        var deadline = DateTime.UtcNow + timeout;
        while (true)
        {
            if (File.Exists(_markerFile))
            {
                var lines = File.ReadAllLines(_markerFile);
                if (Array.IndexOf(lines, "child") >= 0 && Array.IndexOf(lines, "parent") >= 0)
                    return lines;
            }
            if (DateTime.UtcNow >= deadline)
                return File.Exists(_markerFile) ? File.ReadAllLines(_markerFile) : Array.Empty<string>();
            Thread.Sleep(20);
        }
    }
}
