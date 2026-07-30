using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class SmtpDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("smtp", obj["type"]);
        Assert.Equal(1025, obj["port"]);
        Assert.Equal(8025, obj["ui_port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithPort(1125).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(1125, obj["port"]);
    }

    [Fact]
    public void Build_CustomUiPort_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithUiPort(8125).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(8125, obj["ui_port"]);
    }

    [Fact]
    public void Build_WithStarttls_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithStarttls().Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("starttls", obj["tls_mode"]);
    }

    [Fact]
    public void Build_WithImplicitTls_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithImplicitTls().Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("implicit", obj["tls_mode"]);
    }

    [Fact]
    public void Build_WithImage_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithImage("custom:tag").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("custom:tag", obj["image"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new SmtpDependencyBuilder("test").WithContainerName("my-container").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-container", obj["container_name"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new SmtpDependencyBuilder("test").Build();
        Assert.StartsWith("arena-smtp-", dep.Identifier);
    }
}
