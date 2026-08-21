using ArenaDotnet.Xunit.Component;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ExecutableComponentBuilderSerializationTest
{
    [Fact]
    public void Build_WithExecutablePath_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("exec", obj["type"]);
        Assert.Equal("./myapp", obj["executable_path"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_WithEnvVar_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
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
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithRuntimeArg("web_app_port", "8080")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        var runtimeArgs = obj["runtime_args"];
        Assert.NotNull(runtimeArgs);
        var arg = Assert.Single(runtimeArgs);
        Assert.Equal("web_app_port", arg["name"]);
        Assert.Equal("8080", arg["value"]);
    }

    [Fact]
    public void Build_WithSourcePathAndBuildTool_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithSourcePath("./src")
            .WithBuildTool(BuildTool.Cargo)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("./src", obj["source_path"]);
        Assert.Equal("cargo", obj["build_tool"]);
    }

    [Fact]
    public void Build_WithCustomBuildTool_SerializesCommandAndArgs()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithBuildTool(BuildTool.Custom("make", new[] { "release" }))
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        var buildTool = obj["build_tool"];
        Assert.NotNull(buildTool);
        Assert.Equal("make", buildTool["command"]);
        Assert.Equal("release", buildTool["args"]?[0]);
    }

    [Fact]
    public void Build_WithBuildToolCustom_SerializesCommandAndArgs()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithBuildToolCustom("make", new[] { "release" })
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        var buildTool = obj["build_tool"];
        Assert.NotNull(buildTool);
        Assert.Equal("make", buildTool["command"]);
        Assert.Equal("release", buildTool["args"]?[0]);
    }

    [Fact]
    public void Build_WithReadinessCheck_SerializesHttpKind()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8080/health", 5000)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        var readinessChecks = obj["readiness_checks"];
        Assert.NotNull(readinessChecks);
        var check = Assert.Single(readinessChecks);
        Assert.Equal("http", check["kind"]);
        Assert.Equal("http://127.0.0.1:8080/health", check["target"]);
        Assert.Equal(5000, check["timeout_ms"]);
    }

    [Fact]
    public void Build_WithoutReadinessCheck_OmitsReadinessChecksField()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Null(obj["readiness_checks"]);
    }

    [Fact]
    public void Build_WithoutExecutablePath_Throws()
    {
        var builder = new ExecutableComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        Assert.StartsWith("arena-exec-", comp.Identifier);
    }
}
