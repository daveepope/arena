using System.Linq;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Playbook;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class MatchBuilderTest
{
    [Fact]
    public void Build_WithName_ReturnsMatch()
    {
        var match = new MatchBuilder("my-match").Build();
        Assert.Equal("my-match", match.Name);
    }

    [Fact]
    public void Build_WithNullName_Throws()
    {
        Assert.Throws<System.ArgumentNullException>(() => new MatchBuilder(null!));
    }

    [Fact]
    public void Build_WithNetwork_SetsNetwork()
    {
        var match = new MatchBuilder("my-match")
            .WithNetwork("my-network")
            .Build();
        Assert.Equal("my-network", match.Network);
    }

    [Fact]
    public void Build_WithDependency_AddsToDependencies()
    {
        var dep = new HttpDependencyBuilder("http").Build();
        var match = new MatchBuilder("my-match")
            .AddDependency(dep)
            .Build();
        Assert.Single(match.Dependencies);
    }

    [Fact]
    public void Build_WithComponent_AddsToComponents()
    {
        var comp = new ContainerizedComponentBuilder("comp")
            .WithContainerfile("./Dockerfile")
            .Build();
        var match = new MatchBuilder("my-match")
            .AddComponent(comp)
            .Build();
        Assert.Single(match.Components);
    }

    [Fact]
    public void Build_ForFfi_SerializesCorrectJson()
    {
        var dep = new HttpDependencyBuilder("http").Build();
        var match = new MatchBuilder("my-match")
            .AddDependency(dep)
            .Build();
        var json = match.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-match", obj["match_name"]);
        Assert.NotNull(obj["dependencies"]);
    }

    [Fact]
    public void Build_ForFfiWithNetwork_IncludesNetwork()
    {
        var match = new MatchBuilder("my-match")
            .WithNetwork("my-network")
            .Build();
        var json = match.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-network", obj["network"]);
    }

    [Fact]
    public void RegisterPlaybook_UnmanagedPlaybook_AddsToPlaybooks()
    {
        var playbook = new UnmanagedPlaybookTest.TestUnmanagedPlaybook("seed-id");
        var match = new MatchBuilder("my-match")
            .RegisterPlaybook(playbook, false)
            .Build();
        Assert.Single(match.Playbooks);
    }

    [Fact]
    public void RegisterPlaybook_UnmanagedPlaybookOmittedFromFfi_ExcludesFromPlaybooksArray()
    {
        var playbook = new UnmanagedPlaybookTest.TestUnmanagedPlaybook("seed-id");
        var match = new MatchBuilder("my-match")
            .RegisterPlaybook(playbook, false)
            .Build();
        var json = match.ForFfi();
        var obj = JObject.Parse(json);
        Assert.True(obj["playbooks"] == null || !obj["playbooks"]!.Any());
    }

    [Fact]
    public void RegisterPlaybook_UnmanagedPlaybookWithExecOnDependencyStart_Throws()
    {
        var playbook = new UnmanagedPlaybookTest.TestUnmanagedPlaybook("seed-id");
        Assert.Throws<System.ArgumentException>(
            () => new MatchBuilder("my-match").RegisterPlaybook(playbook, true));
    }

    [Fact]
    public void RegisterPlaybook_BarePlaybookInterface_Throws()
    {
        var playbook = new BarePlaybook();
        Assert.Throws<System.ArgumentException>(
            () => new MatchBuilder("my-match").RegisterPlaybook(playbook, false));
    }

    [Fact]
    public void RegisterPlaybook_NullPlaybook_ThrowsArgumentNullException()
    {
        Assert.Throws<System.ArgumentNullException>(
            () => new MatchBuilder("my-match").RegisterPlaybook(null!, false));
    }

    private class BarePlaybook : Playbook.IPlaybook
    {
        public string Identifier => "bare-id";
        public ActivePlaybook Run(OpenArena arena) => throw new System.NotImplementedException();
    }
}
