using ArenaXunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class TemporalDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new TemporalDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("temporal", obj["type"]);
        Assert.Equal(7233, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new TemporalDependencyBuilder("test").WithPort(7333).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(7333, obj["port"]);
    }

    [Fact]
    public void Build_DefaultUiPort_SerializesCorrectJson()
    {
        var dep = new TemporalDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(8233, obj["ui_port"]);
    }

    [Fact]
    public void Build_CustomUiPort_SerializesCorrectJson()
    {
        var dep = new TemporalDependencyBuilder("test").WithUiPort(9333).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(9333, obj["ui_port"]);
    }

    [Fact]
    public void Build_WithImageAndContainerName_SerializesCorrectJson()
    {
        var dep = new TemporalDependencyBuilder("test")
            .WithImage("1.8.0")
            .WithImageName("temporalio/temporal")
            .WithContainerName("my-temporal")
            .Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("1.8.0", obj["image"]);
        Assert.Equal("temporalio/temporal", obj["image_name"]);
        Assert.Equal("my-temporal", obj["container_name"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new TemporalDependencyBuilder("test").Build();
        Assert.StartsWith("arena-temporal-", dep.Identifier);
    }
}
