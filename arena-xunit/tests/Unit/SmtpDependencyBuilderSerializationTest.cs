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
    public void build_default_port_serializes_correct_json()
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
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithPort(1125).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(1125, obj["port"]);
    }

    [Fact]
    public void build_custom_ui_port_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithUiPort(8125).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(8125, obj["ui_port"]);
    }

    [Fact]
    public void build_with_starttls_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithStarttls().Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("starttls", obj["tls_mode"]);
    }

    [Fact]
    public void build_with_implicit_tls_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithImplicitTls().Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("implicit", obj["tls_mode"]);
    }

    [Fact]
    public void build_with_image_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithImage("custom:tag").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("custom:tag", obj["image"]);
    }

    [Fact]
    public void build_with_container_name_serializes_correct_json()
    {
        var dep = new SmtpDependencyBuilder("test").WithContainerName("my-container").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-container", obj["container_name"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new SmtpDependencyBuilder("test").Build();
        Assert.StartsWith("arena-smtp-", dep.Identifier);
    }
}
