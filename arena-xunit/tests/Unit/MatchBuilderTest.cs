using System.Linq;
using ArenaXunit.Component;
using ArenaXunit.Dep;
using ArenaXunit.Topology;
using ArenaXunit.Playbook;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class MatchBuilderTest
{
    [Fact]
    public void build_with_name_returns_match()
    {
        var match = new MatchBuilder("my-match").Build();
        Assert.Equal("my-match", match.Name);
    }

    [Fact]
    public void build_with_null_name_throws()
    {
        Assert.Throws<System.ArgumentNullException>(() => new MatchBuilder(null));
    }

    [Fact]
    public void build_with_network_sets_network()
    {
        var match = new MatchBuilder("my-match")
            .WithNetwork("my-network")
            .Build();
        Assert.Equal("my-network", match.Network);
    }

    [Fact]
    public void build_with_dependency_adds_to_dependencies()
    {
        var dep = new HttpDependencyBuilder("http").Build();
        var match = new MatchBuilder("my-match")
            .AddDependency(dep)
            .Build();
        Assert.Single(match.Dependencies);
    }

    [Fact]
    public void build_with_component_adds_to_components()
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
    public void build_for_ffi_serializes_correct_json()
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
    public void build_for_ffi_with_network_includes_network()
    {
        var match = new MatchBuilder("my-match")
            .WithNetwork("my-network")
            .Build();
        var json = match.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-network", obj["network"]);
    }
}
