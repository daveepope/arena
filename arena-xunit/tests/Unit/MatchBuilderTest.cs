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
        Assert.Throws<System.ArgumentNullException>(() => new MatchBuilder(null));
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
}
