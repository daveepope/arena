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
        var envVars = obj["env_vars"];
        Assert.NotNull(envVars);
        Assert.Equal("value", envVars["KEY"]);
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
        var runtimeArgs = obj["runtime_args"];
        Assert.NotNull(runtimeArgs);
        var arg = Assert.Single(runtimeArgs);
        Assert.Equal("flag", arg["name"]);
        Assert.Equal("arg1", arg["value"]);
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
        var portMappings = obj["port_mappings"];
        Assert.NotNull(portMappings);
        var mapping = Assert.Single(portMappings);
        Assert.Equal(8080, mapping["host_port"]);
        Assert.Equal(80, mapping["container_port"]);
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
        var hostMappings = obj["host_mappings"];
        Assert.NotNull(hostMappings);
        var mapping = Assert.Single(hostMappings);
        Assert.Equal("host.docker.internal:host-gateway", mapping);
    }

    [Fact]
    public void Build_WithVolumeMapping_SerializesHostAndContainerPath()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithVolumeMapping("/host/path", "/container/path")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        var volumeMappings = obj["volume_mappings"];
        Assert.NotNull(volumeMappings);
        var mapping = Assert.Single(volumeMappings);
        Assert.Equal("/host/path", mapping["host_path"]);
        Assert.Equal("/container/path", mapping["container_path"]);
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
        var readinessChecks = obj["readiness_checks"];
        Assert.NotNull(readinessChecks);
        var check = Assert.Single(readinessChecks);
        Assert.Equal("http", check["kind"]);
    }

    [Fact]
    public void Build_WithoutContainerfile_Throws()
    {
        var builder = new ContainerizedComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_WithContainerfileAndImage_Throws()
    {
        var builder = ContainerizedComponentBuilder.FromImage("test", "postgres:18-bookworm")
            .WithContainerfile("./Dockerfile");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_FromImage_SerializesImageWithoutContainerfile()
    {
        var comp = ContainerizedComponentBuilder.FromImage("test", "postgres:18-bookworm").Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("container", obj["type"]);
        Assert.Equal("postgres:18-bookworm", obj["image"]);
        Assert.Null(obj["containerfile"]);
    }

    [Fact]
    public void Build_WithPlatform_SerializesPlatform()
    {
        var comp = new ContainerizedComponentBuilder("test")
            .WithContainerfile("./Dockerfile")
            .WithPlatform("linux/arm64")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("linux/arm64", obj["platform"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var comp = new ContainerizedComponentBuilder("test").WithContainerfile("./Dockerfile").Build();
        Assert.StartsWith("arena-container-", comp.Identifier);
    }
}
