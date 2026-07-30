using ArenaXunit.Component;
using ArenaXunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ContainerizedComponentBuilderSerializationTest
{
    [Fact]
    public void Build_WithContainerfile_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test").WithContainerfile("./Dockerfile").Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("container", obj["type"]);
        Assert.Equal("./Dockerfile", obj["containerfile"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_WithEnv_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithEnv("KEY", "value")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["env"]);
        Assert.Equal("value", obj["env"]["key"]);
    }

    [Fact]
    public void Build_WithArgs_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithArgs("--flag", "arg1")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["args"]);
        Assert.Equal("--flag", obj["args"][0]);
        Assert.Equal("arg1", obj["args"][1]);
    }

    [Fact]
    public void Build_WithoutContainerfile_Throws()
    {
        var builder = new ContainerizedComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_IdentifierMatchesPattern()
    {
        var comp = new ContainerizedComponentBuilder("test").WithContainerfile("./Dockerfile").Build();
        Assert.StartsWith("arena-container-", comp.Identifier);
    }
}
