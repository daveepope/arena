using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class PostgresDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new PostgresDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("postgres", obj["type"]);
        Assert.Equal(5432, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new PostgresDependencyBuilder("test").WithPort(5532).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(5532, obj["port"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new PostgresDependencyBuilder("test").Build();
        Assert.StartsWith("arena-postgres-", dep.Identifier);
    }

    [Fact]
    public void Build_WithImageName_SerializesCorrectJson()
    {
        var dep = new PostgresDependencyBuilder("test").WithImageName("postgres").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("postgres", obj["image_name"]);
    }

    [Fact]
    public void Build_WithImage_SerializesCorrectJson()
    {
        var dep = new PostgresDependencyBuilder("test").WithImage("16-alpine").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("16-alpine", obj["image"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new PostgresDependencyBuilder("test").WithContainerName("my-postgres").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-postgres", obj["container_name"]);
    }
}
