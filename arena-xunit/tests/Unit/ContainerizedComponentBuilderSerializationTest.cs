using ArenaDotnet.Xunit.Component;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

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
    public void Build_WithEnvVar_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithEnvVar("KEY", "value")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["env_vars"]);
        Assert.Equal("value", obj["env_vars"]["KEY"]);
    }

    [Fact]
    public void Build_WithRuntimeArg_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithRuntimeArg("flag", "arg1")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["runtime_args"]);
        Assert.Equal("flag", obj["runtime_args"][0]["name"]);
        Assert.Equal("arg1", obj["runtime_args"][0]["value"]);
    }

    [Fact]
    public void Build_WithBuildContextImageTagAndNetwork_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithBuildContext("./ctx")
            .WithImageTag("v1")
            .WithNetwork("test-net")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("./ctx", obj["build_context"]);
        Assert.Equal("v1", obj["image_tag"]);
        Assert.Equal("test-net", obj["network"]);
    }

    [Fact]
    public void Build_WithPortMapping_SerializesHostAndContainerPort()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithPortMapping(8080, 80)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["port_mappings"]);
        Assert.Equal(8080, obj["port_mappings"][0]["host_port"]);
        Assert.Equal(80, obj["port_mappings"][0]["container_port"]);
    }

    [Fact]
    public void Build_WithHostMapping_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithHostMapping("host.docker.internal:host-gateway")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["host_mappings"]);
        Assert.Equal("host.docker.internal:host-gateway", obj["host_mappings"][0]);
    }

    [Fact]
    public void Build_WithBindMount_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithBindMount("/host/data", "/mnt/data", true)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["mounts"]);
        Assert.Equal("bind", obj["mounts"][0]["type"]);
        Assert.Equal("/host/data", obj["mounts"][0]["source"]);
        Assert.Equal("/mnt/data", obj["mounts"][0]["container_path"]);
        Assert.Equal(true, obj["mounts"][0]["read_only"]);
    }

    [Fact]
    public void Build_WithBindMount_DefaultsReadOnlyToFalse()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithBindMount("/host/data", "/mnt/data")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(false, obj["mounts"][0]["read_only"]);
    }

    [Fact]
    public void Build_WithVolumeMount_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithVolumeMount("my-volume", "/mnt/data", true)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["mounts"]);
        Assert.Equal("volume", obj["mounts"][0]["type"]);
        Assert.Equal("my-volume", obj["mounts"][0]["source"]);
        Assert.Equal("/mnt/data", obj["mounts"][0]["container_path"]);
        Assert.Equal(true, obj["mounts"][0]["read_only"]);
    }

    [Fact]
    public void Build_WithTmpfsMount_SerializesCorrectJson()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithTmpfsMount("/mnt/data", 1024)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["mounts"]);
        Assert.Equal("tmpfs", obj["mounts"][0]["type"]);
        Assert.Equal("/mnt/data", obj["mounts"][0]["container_path"]);
        Assert.Equal(1024, obj["mounts"][0]["size_bytes"]);
    }

    [Fact]
    public void Build_WithTmpfsMount_NoSizeBytesArgOmitsSizeBytes()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithTmpfsMount("/mnt/data")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Null(obj["mounts"][0]["size_bytes"]);
    }

    [Fact]
    public void Build_WithoutMounts_SerializesEmptyList()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Empty(obj["mounts"]);
    }

    [Fact]
    public void Build_WithReadinessCheck_SerializesHttpKind()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8080/health")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["readiness_checks"]);
        Assert.Equal("http", obj["readiness_checks"][0]["kind"]);
    }

    [Fact]
    public void Build_WithoutContainerfile_Throws()
    {
        var builder = new ContainerizedComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var comp = new ContainerizedComponentBuilder("test").WithContainerfile("./Dockerfile").Build();
        Assert.StartsWith("arena-container-", comp.Identifier);
    }
}
